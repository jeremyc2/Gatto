import AppKit
import Sparkle

@MainActor
protocol UpdateClient: AnyObject {
    var canCheckForUpdates: Bool { get }
    func start() throws
    func checkForUpdates()
}
extension SPUUpdater: UpdateClient {}

// A custom Sparkle driver keeps all update UI in GPUI. Sparkle owns archive
// verification, privileged authorization if needed, replacement and relaunch.
@MainActor
@objc(GattoUpdaterBridge)
final class UpdaterBridge: NSObject, SPUUserDriver, SPUUpdaterDelegate {
    private var updater: UpdateClient!
    private var handler: ((Int, NSString) -> Void)?
    private let defaults: UserDefaults
    private var availabilityObservation: NSKeyValueObservation?

    override convenience init() { self.init(client: nil) }
    init(client: UpdateClient?, defaults: UserDefaults = .standard) {
        self.updater = client
        self.defaults = defaults
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
                    DispatchQueue.main.async { [weak self] in self?.resumePendingInstallation() }
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
            emit(4, "Updater unavailable: \(error.localizedDescription)")
        }
    }

    @objc func install() {
        guard updater != nil, !installRequested else { return }
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

    private func resumePendingInstallation() {
        guard installPending, updater.canCheckForUpdates else { return }
        installPending = false
        updater.checkForUpdates()
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
        if installPending {
            // Start from the completion callback before Sparkle's next status probe.
            resumePendingInstallation()
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
        installRequested = false
    }

    func show(_ request: SPUUpdatePermissionRequest,
              reply: @escaping (SUUpdatePermissionResponse) -> Void) {
        reply(SUUpdatePermissionResponse(automaticUpdateChecks: true,
              automaticUpdateDownloading: false, sendSystemProfile: false))
    }
    func showUserInitiatedUpdateCheck(cancellation: @escaping () -> Void) {}
    func showUpdateFound(with appcastItem: SUAppcastItem, state: SPUUserUpdateState,
                         reply: @escaping (SPUUserUpdateChoice) -> Void) {
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
        acknowledgement()
    }
    func showUpdaterError(_ error: Error, acknowledgement: @escaping () -> Void) {
        if installRequested {
            failureShown = true
            emit(3, "Update failed: \(error.localizedDescription). Click to retry.")
        } else {
            emit(5, "Update check failed: \(error.localizedDescription)")
        }
        acknowledgement()
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
