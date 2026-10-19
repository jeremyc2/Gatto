import AppKit
import Sparkle

@MainActor
protocol UpdateClient: AnyObject {
    var canCheckForUpdates: Bool { get }
    func start() throws
    func checkForUpdates()
}
extension SPUUpdater: UpdateClient {}

struct UpdatePrompt {
    let title: String
    let detail: String
    var releaseURL: URL? = nil
    var offersInstall = false
}

// Scheduled checks stay quiet; explicit menu checks use native alerts. Sparkle
// owns archive verification, authorization, replacement and relaunch.
@MainActor
@objc(GattoUpdaterBridge)
final class UpdaterBridge: NSObject, SPUUserDriver, SPUUpdaterDelegate {
    private var updater: UpdateClient!
    private var handler: ((Int, NSString) -> Void)?
    private let defaults: UserDefaults
    private let present: @MainActor (UpdatePrompt, @escaping (Bool) -> Void) -> Void
    private var manualRequested = false
    private var manualPending = false
    private var manualResultShown = false
    private var startupError: String?
    private var availabilityObservation: NSKeyValueObservation?

    override convenience init() { self.init(client: nil) }
    init(client: UpdateClient?, defaults: UserDefaults = .standard,
         present: @escaping @MainActor (UpdatePrompt, @escaping (Bool) -> Void) -> Void = UpdaterBridge.presentAlert) {
        self.updater = client
        self.defaults = defaults
        self.present = present
        super.init()
    }
    private var installRequested = false
    private var installPending = false
    private var failureShown = false
    private var version: String? {
        get { defaults.string(forKey: "GattoAvailableUpdate") }
        set { defaults.set(newValue, forKey: "GattoAvailableUpdate") }
    }

    @objc(startWithHandler:)
    func start(handler: @escaping (Int, NSString) -> Void) {
        self.handler = handler
        let bundle = Bundle.main
        if updater == nil {
            let sparkle = SPUUpdater(hostBundle: bundle, applicationBundle: bundle,
                                     userDriver: self, delegate: self)
            updater = sparkle
            availabilityObservation = sparkle.observe(\.canCheckForUpdates, options: [.new]) { [weak self] _, change in
                if change.newValue == true {
                    DispatchQueue.main.async { [weak self] in self?.resumePendingOperation() }
                }
            }
        }
        do {
            try updater.start()
            if let available = version,
               SUStandardVersionComparator.default.compareVersion(
                available, toVersion: bundle.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "0"
               ) == .orderedDescending {
                emit(1, "Install \(available) and restart Gatto")
            } else {
                version = nil
                emit(0, "")
            }
        } catch {
            startupError = error.localizedDescription
            emit(4, "Updater unavailable: \(error.localizedDescription)")
        }
    }

    // Present outside GPUI's current update: AppKit's modal loop can call back
    // into GPUI. Tests inject a presenter without opening real windows.
    private static func presentAlert(_ prompt: UpdatePrompt, reply: @escaping (Bool) -> Void) {
        DispatchQueue.main.async {
            let alert = NSAlert()
            alert.messageText = prompt.title
            alert.informativeText = prompt.detail
            alert.addButton(withTitle: prompt.offersInstall ? "Update" : "OK")
            if prompt.offersInstall { alert.addButton(withTitle: "Cancel") }
            if let url = prompt.releaseURL {
                let link = NSTextField(labelWithString: "")
                link.attributedStringValue = NSAttributedString(string: "View GitHub release", attributes: [
                    .link: url, .foregroundColor: NSColor.linkColor,
                    .underlineStyle: NSUnderlineStyle.single.rawValue
                ])
                link.isSelectable = true
                link.allowsEditingTextAttributes = true
                link.setFrameSize(NSSize(width: 280, height: 22))
                alert.accessoryView = link
            }
            NSApp.activate(ignoringOtherApps: true)
            reply(alert.runModal() == .alertFirstButtonReturn && prompt.offersInstall)
        }
    }

    @objc func checkForUpdates() {
        guard !installRequested, !manualRequested else { return }
        if let startupError {
            present(UpdatePrompt(title: "Update checking is unavailable", detail: startupError)) { _ in }
            return
        }
        guard updater != nil else { return }
        manualRequested = true
        manualResultShown = false
        if updater.canCheckForUpdates {
            updater.checkForUpdates()
        } else {
            manualPending = true
        }
    }

    @objc func install() {
        guard updater != nil, !installRequested else { return }
        // An existing Update control still grants consent in one click.
        manualRequested = false
        manualPending = false
        installRequested = true
        failureShown = false
        emit(2, "Downloading update; Gatto will restart when ready")
        if updater.canCheckForUpdates {
            updater.checkForUpdates()
        } else {
            // A cached badge can be clicked while an information check is in flight.
            // Preserve that single click and begin as soon as its session ends.
            installPending = true
        }
    }

    private func resumePendingOperation() {
        guard updater.canCheckForUpdates else { return }
        if installPending {
            installPending = false
            updater.checkForUpdates()
        } else if manualPending {
            manualPending = false
            updater.checkForUpdates()
        }
    }

    private func emit(_ state: Int, _ message: String) { handler?(state, message as NSString) }

    func updater(_ updater: SPUUpdater, didFindValidUpdate item: SUAppcastItem) {
        version = item.versionString
        if !installRequested { emit(1, "Install \(item.displayVersionString) and restart Gatto") }
    }

    func updater(_ updater: SPUUpdater, didFinishUpdateCycleFor updateCheck: SPUUpdateCheck, error: Error?) {
        finishCycle(error: error)
    }

    func finishCycle(error: Error?) {
        if installPending || manualPending {
            // Start from the completion callback before Sparkle's next status probe.
            resumePendingOperation()
            return
        }
        if let error = error as NSError? {
            if error.domain == SUSparkleErrorDomain && error.code == SUError.noUpdateError.rawValue {
                version = nil
                if !failureShown { emit(0, "") }
            } else if !installRequested {
                // Checking silently must never raise an alert or overwrite a cached badge.
                emit(5, "Update check failed: \(error.localizedDescription)")
            }
        } else if !installRequested && version == nil {
            emit(0, "")
        }
        if manualRequested && !manualResultShown, let error = error as NSError? {
            let noUpdate = error.domain == SUSparkleErrorDomain && error.code == SUError.noUpdateError.rawValue
            showManualMessage(title: noUpdate ? "No update available" : "Could not check for updates",
                              detail: error.localizedDescription)
        }
        installRequested = false
        manualRequested = false
    }

    private func showManualMessage(title: String, detail: String,
                                   acknowledgement: @escaping () -> Void = {}) {
        manualResultShown = true
        present(UpdatePrompt(title: title, detail: detail)) { _ in acknowledgement() }
    }

    // Kept independent of Sparkle's item type so consent and cancellation are
    // exercised by the native harness without networking or a live feed.
    func confirmUpdate(displayVersion: String, releaseURL: URL?, informationOnly: Bool,
                       reply: @escaping (SPUUserUpdateChoice) -> Void) {
        manualResultShown = true
        present(UpdatePrompt(title: "Gatto \(displayVersion) is available",
                             detail: informationOnly
                                ? "This release cannot be installed in-app. Visit the GitHub release for details."
                                : "Update and restart Gatto? Finish any active uploads before continuing.",
                             releaseURL: releaseURL, offersInstall: !informationOnly)) { [weak self] install in
            // End this check before routing consent back through Rust's current
            // upload guards. Cancel dismisses without skipping future reminders.
            reply(.dismiss)
            self?.manualRequested = false
            if install && !informationOnly { self?.emit(6, "") }
        }
    }

    func show(_ request: SPUUpdatePermissionRequest,
              reply: @escaping (SUUpdatePermissionResponse) -> Void) {
        reply(SUUpdatePermissionResponse(automaticUpdateChecks: true,
              automaticUpdateDownloading: false, sendSystemProfile: false))
    }
    func showUserInitiatedUpdateCheck(cancellation: @escaping () -> Void) {}
    func showUpdateFound(with appcastItem: SUAppcastItem, state: SPUUserUpdateState,
                         reply: @escaping (SPUUserUpdateChoice) -> Void) {
        if manualRequested && !manualPending {
            confirmUpdate(displayVersion: appcastItem.displayVersionString,
                          releaseURL: appcastItem.infoURL ?? appcastItem.releaseNotesURL,
                          informationOnly: appcastItem.isInformationOnlyUpdate, reply: reply)
            return
        }
        guard installRequested, !appcastItem.isInformationOnlyUpdate else {
            if installRequested && appcastItem.isInformationOnlyUpdate {
                failureShown = true
                emit(3, "This release cannot be installed in-app. See the GitHub release notes.")
            }
            reply(.dismiss)
            return
        }
        installPending = false
        reply(.install)
    }
    func showUpdateReleaseNotes(with downloadData: SPUDownloadData) {}
    func showUpdateReleaseNotesFailedToDownloadWithError(_ error: Error) {}
    func showUpdateNotFoundWithError(_ error: Error, acknowledgement: @escaping () -> Void) {
        version = nil
        emit(0, "")
        if manualRequested && !manualPending {
            showManualMessage(title: "No update available", detail: error.localizedDescription,
                              acknowledgement: acknowledgement)
        } else {
            acknowledgement()
        }
    }
    func showUpdaterError(_ error: Error, acknowledgement: @escaping () -> Void) {
        if installRequested {
            failureShown = true
            emit(3, "Update failed: \(error.localizedDescription). Click to retry.")
        } else {
            emit(5, "Update check failed: \(error.localizedDescription)")
        }
        if manualRequested && !manualPending {
            showManualMessage(title: "Could not check for updates", detail: error.localizedDescription,
                              acknowledgement: acknowledgement)
        } else {
            acknowledgement()
        }
    }
    func showDownloadInitiated(cancellation: @escaping () -> Void) {
        emit(2, "Downloading update; Gatto will restart when ready")
    }
    func showDownloadDidReceiveExpectedContentLength(_ expectedContentLength: UInt64) {}
    func showDownloadDidReceiveData(ofLength length: UInt64) {}
    func showDownloadDidStartExtractingUpdate() { emit(2, "Preparing update and restart") }
    func showExtractionReceivedProgress(_ progress: Double) {}
    func showReady(toInstallAndRelaunch reply: @escaping (SPUUserUpdateChoice) -> Void) {
        reply(installRequested ? .install : .skip)
    }
    func showInstallingUpdate(withApplicationTerminated applicationTerminated: Bool,
                             retryTerminatingApplication: @escaping () -> Void) {
        emit(2, "Installing update and restarting Gatto")
    }
    func showUpdateInstalledAndRelaunched(_ relaunched: Bool, acknowledgement: @escaping () -> Void) {
        version = nil
        emit(0, "")
        acknowledgement()
    }
    func dismissUpdateInstallation() {}
}
