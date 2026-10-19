import AppKit
import Sparkle

@MainActor
final class FakeClient: UpdateClient {
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
        bridge.start { state, _ in events.append(state) }
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
        bridge.start { state, _ in events.append(state) }
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
        failed.start { _, _ in }
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

}
