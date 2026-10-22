import AppKit
import Sparkle

@MainActor
final class FakeClient: UpdateClient {
    var sessionInProgress: Bool { !canCheckForUpdates }
    var informationChecks = 0
    func checkForUpdateInformation() { informationChecks += 1 }
    var canCheckForUpdates = true
    var installationChecks = 0
    var startupError: Error?
    func start() throws {
        if let startupError { throw startupError }
    }
    func checkForUpdates() { installationChecks += 1 }
}

@main
struct UpdaterTests {
    @MainActor static func main() {
        let suite = "GattoUpdaterTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let client = FakeClient()
        let bridge = UpdaterBridge(client: client, defaults: defaults)
        var events: [Int] = []
        bridge.start { _, state, _ in events.append(state) }
        assert(client.installationChecks == 0)
        bridge.showUpdaterError(NSError(domain: "test", code: 1), acknowledgement: {})
        assert(events.last == 5, "background failures belong in diagnostics, not an install prompt")
        // Merely reaching ready-to-install never grants consent. Clicking does,
        // and one click proceeds through download and relaunch without another dialog.
        var choice: SPUUserUpdateChoice?
        bridge.showReady(toInstallAndRelaunch: { choice = $0 })
        assert(choice == .skip)
        bridge.install()
        bridge.install()
        assert(client.installationChecks == 1)
        bridge.showReady(toInstallAndRelaunch: { choice = $0 })
        assert(choice == .install)
        // Clicking a cached badge during a background check must not lose the click.
        let busy = FakeClient()
        busy.canCheckForUpdates = false
        let queued = UpdaterBridge(client: busy, defaults: defaults)
        queued.install()
        assert(busy.installationChecks == 0)
        busy.canCheckForUpdates = true
        queued.finishCycle(error: nil)
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        assert(busy.installationChecks == 1)
        // A failed install permits another explicit click; no auto retry or install.
        queued.finishCycle(error: NSError(domain: "test", code: 1))
        assert(busy.installationChecks == 1)
        queued.install()
        assert(busy.installationChecks == 2)
        testManualChecks(defaults: defaults)
        testChannels(defaults: defaults)
        print("Passed: quiet background checks, one-click installs, manual checks, confirmation, cancellation and retry")
    }

    @MainActor static func testManualChecks(defaults: UserDefaults) {
        let client = FakeClient()
        var prompts: [UpdatePrompt] = []
        var respond: ((Bool) -> Void)?
        let bridge = UpdaterBridge(client: client, defaults: defaults) { prompt, reply in
            prompts.append(prompt)
            respond = reply
        }
        var events: [Int] = []
        bridge.start { _, state, _ in events.append(state) }
        let error = NSError(domain: "test", code: 1)
        // A menu check is explicit, deduplicated, and never installs by itself.
        bridge.checkForUpdates()
        bridge.checkForUpdates()
        assert(client.installationChecks == 1)
        var choice: SPUUserUpdateChoice?
        bridge.showReady(toInstallAndRelaunch: { choice = $0 })
        assert(choice == .skip)
        let release = URL(string: "https://github.com/jeremyc2/Gatto/releases/tag/v99.0.0")!
        bridge.confirmUpdate(displayVersion: "99.0.0", releaseURL: release,
                             informationOnly: false) { choice = $0 }
        assert(prompts.last?.title == "Gatto 99.0.0 is available")
        assert(prompts.last?.releaseURL == release && prompts.last?.offersInstall == true)
        respond?(false)
        assert(choice == .dismiss && !events.contains(6))
        bridge.finishCycle(error: nil)
        bridge.checkForUpdates()
        bridge.confirmUpdate(displayVersion: "99.0.0", releaseURL: release,
                             informationOnly: false) { choice = $0 }
        respond?(true)
        assert(choice == .dismiss && events.last == 6, "consent must go through Rust's upload guards")
        assert(!events.contains(2), "checking and confirming must not bypass the installation action")
        bridge.finishCycle(error: nil)
        bridge.checkForUpdates()
        let requests = events.filter { $0 == 6 }.count
        bridge.confirmUpdate(displayVersion: "99.0.0", releaseURL: release,
                             informationOnly: true) { choice = $0 }
        assert(prompts.last?.offersInstall == false)
        respond?(true)
        assert(events.filter { $0 == 6 }.count == requests)
        bridge.finishCycle(error: nil)

        bridge.checkForUpdates()
        var acknowledged = false
        bridge.showUpdateNotFoundWithError(error) { acknowledged = true }
        assert(prompts.last?.title == "No update available" && !acknowledged)
        respond?(false)
        assert(acknowledged)
        bridge.finishCycle(error: error)
        let count = prompts.count
        bridge.showUpdaterError(error, acknowledgement: {})
        assert(prompts.count == count, "background errors must stay quiet")

        bridge.checkForUpdates()
        bridge.showUpdaterError(error, acknowledgement: {})
        assert(prompts.last?.title == "Could not check for updates")
        respond?(false)
        bridge.finishCycle(error: error)
        assert(prompts.count == count + 1, "cycle completion must not duplicate the error alert")

        // A menu click during a background session waits for a fresh check.
        client.canCheckForUpdates = false
        let checks = client.installationChecks
        bridge.checkForUpdates()
        bridge.checkForUpdates()
        bridge.showUpdaterError(error, acknowledgement: {})
        assert(prompts.count == count + 1)
        bridge.finishCycle(error: nil)
        assert(client.installationChecks == checks)
        client.canCheckForUpdates = true
        bridge.finishCycle(error: nil)
        assert(client.installationChecks == checks + 1)
        bridge.showUpdateNotFoundWithError(error, acknowledgement: {})
        assert(prompts.last?.title == "No update available")
        respond?(false)
        bridge.finishCycle(error: nil)

        // Existing badge consent also works while a manual check is underway.
        bridge.checkForUpdates()
        bridge.install()
        bridge.showReady(toInstallAndRelaunch: { choice = $0 })
        assert(choice == .install && events.last == 2)

        let unavailable = FakeClient()
        unavailable.startupError = error
        let failed = UpdaterBridge(client: unavailable, defaults: defaults) { prompt, reply in
            prompts.append(prompt)
            reply(false)
        }
        failed.start { _, _, _ in }
        failed.checkForUpdates()
        assert(prompts.last?.title == "Update checking is unavailable")
        assert(unavailable.installationChecks == 0)

        let fallback = UpdaterBridge(client: client, defaults: defaults) { prompt, reply in
            prompts.append(prompt)
            reply(false)
        }
        fallback.checkForUpdates()
        fallback.finishCycle(error: NSError(domain: SUSparkleErrorDomain, code: Int(SUError.noUpdateError.rawValue)))
        assert(prompts.last?.title == "No update available")
    }

    @MainActor static func testChannels(defaults: UserDefaults) {
        let comparator = SUStandardVersionComparator.default
        assert(comparator.compareVersion("0.17.0b10", toVersion: "0.17.0b9") == .orderedDescending)
        assert(comparator.compareVersion("0.17.0", toVersion: "0.17.0b10") == .orderedDescending)
        assert(!ReleaseChannel.stable.accepts(version: "0.18.0b1", channel: "beta"))
        assert(!ReleaseChannel.stable.accepts(version: "0.18.0b1", channel: nil))
        assert(ReleaseChannel.beta.accepts(version: "0.18.0", channel: nil))
        assert(ReleaseChannel.beta.allowedChannels == ["beta"] && ReleaseChannel.stable.allowedChannels.isEmpty)
        assert(ReleaseChannel.stable.feedURL != ReleaseChannel.beta.feedURL)
        defaults.set("0.18.0b1", forKey: "GattoAvailableUpdate")
        defaults.set("beta", forKey: "GattoAvailableUpdateChannel")
        let client = FakeClient()
        var respond: ((Bool) -> Void)?
        let bridge = UpdaterBridge(client: client, defaults: defaults, installedVersion: "0.17.0b1") { _, reply in respond = reply }
        bridge.configureChannel("beta", generation: 0)
        var events: [(Int, Int, String)] = []
        bridge.start { generation, state, message in events.append((generation, state, message as String)) }
        assert(events.last?.1 == 1, "only matching channel caches restore")
        bridge.confirmUpdate(displayVersion: "0.18.0b1", releaseURL: nil, informationOnly: false) { _ in }
        client.canCheckForUpdates = false
        bridge.configureChannel("stable", generation: 1)
        let count = events.count
        respond?(true)
        bridge.showUpdaterError(NSError(domain: "old-channel", code: 1), acknowledgement: {})
        bridge.finishCycle(error: nil)
        assert(events.count == count, "stale prompts and checks cannot resurrect old offers or consent")
        assert(defaults.string(forKey: "GattoAvailableUpdate") == nil)
        assert(client.informationChecks == 0 && client.installationChecks == 0)
        client.canCheckForUpdates = true
        bridge.finishCycle(error: nil)
        assert(client.informationChecks == 1 && client.installationChecks == 0, "switching only probes")
        bridge.recordStableTarget("0.16.3", version: "0.16.3")
        bridge.finishCycle(error: NSError(domain: SUSparkleErrorDomain, code: Int(SUError.noUpdateError.rawValue)))
        assert(events.last?.0 == 1 && events.last?.1 == 0)
        assert(events.last!.2.contains("0.17.0-beta.1") && events.last!.2.contains("0.16.3"))
        var choice: SPUUserUpdateChoice?
        bridge.showReady(toInstallAndRelaunch: { choice = $0 })
        assert(choice == .skip, "an older stable never downloads or installs")
        bridge.beginCycle()
        bridge.recordStableTarget("0.17.0", version: "0.17.0")
        bridge.finishCycle(error: nil)
        assert(events.last?.2 == "", "Stable catching up clears the wait")
        bridge.configureChannel("beta", generation: 2)
        assert(client.informationChecks == 2 && client.installationChecks == 0)
        bridge.install()
        let installedGeneration = bridge.generation
        bridge.configureChannel("stable", generation: 3)
        assert(bridge.generation == installedGeneration, "channel changes cannot interrupt an authorized installation")
        // A beta cache never appears after restarting on Stable.
        defaults.set("0.18.0b1", forKey: "GattoAvailableUpdate")
        defaults.set("beta", forKey: "GattoAvailableUpdateChannel")
        let stable = UpdaterBridge(client: FakeClient(), defaults: defaults, installedVersion: "0.16.3")
        var states: [Int] = []
        stable.start { _, state, _ in states.append(state) }
        assert(states == [0] && defaults.string(forKey: "GattoAvailableUpdate") == nil)
        print("Passed: channel isolation, restart caches, stale consent/checks, deferred refresh and return-to-Stable policy")
    }

}
