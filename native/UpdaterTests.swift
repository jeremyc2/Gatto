import AppKit
import Sparkle

@MainActor
final class FakeClient: UpdateClient {
    var canCheckForUpdates = true
    var installationChecks = 0
    func start() throws {}
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
        print("Passed: quiet background failures, busy session, click-only installation and retry")
    }
}
