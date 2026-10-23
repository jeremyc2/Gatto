// Runs the shipped Automator action against a disposable application. Never
// registers a real Gatto bundle or installs anything in ~/Library/Services.
import AppKit
import Automator
import Foundation

final class Receiver: NSObject, NSApplicationDelegate {
    var selections = [[String]]()
    func application(_ application: NSApplication, open urls: [URL]) {
        selections.append(urls.map(\.path))
        let output = Bundle.main.object(forInfoDictionaryKey: "GattoTestOutput") as! String
        let record: [String: Any] = ["pid": ProcessInfo.processInfo.processIdentifier, "selections": selections]
        try! JSONSerialization.data(withJSONObject: record).write(to: URL(fileURLWithPath: output), options: .atomic)
    }
}

func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw NSError(domain: "FinderActionTests", code: 1, userInfo: [NSLocalizedDescriptionKey: message]) }
}

func run(_ executable: String, _ arguments: [String]) throws {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: executable)
    process.arguments = arguments
    try process.run()
    process.waitUntilExit()
    try require(process.terminationStatus == 0, "Failed: \(executable) \(arguments)")
}

func verify() throws {
    let files = FileManager.default
    let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        .appendingPathComponent("dist/gatto-finder-\(UUID().uuidString)")
    try files.createDirectory(at: root, withIntermediateDirectories: true)
    let app = root.appendingPathComponent("Receiver.app")
    let contents = app.appendingPathComponent("Contents")
    let binary = contents.appendingPathComponent("MacOS/receiver")
    let resources = contents.appendingPathComponent("Resources")
    let source = URL(fileURLWithPath: CommandLine.arguments[1])
    let output = root.appendingPathComponent("received.json")
    let identifier = "com.jeremy-chandler.gatto.finder-test.\(UUID().uuidString)"
    let register = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
    defer {
        if let data = try? Data(contentsOf: output),
           let record = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let pid = record["pid"] as? Int32,
           let running = NSRunningApplication(processIdentifier: pid), running.bundleURL?.path == app.path {
            running.terminate()
        }
        try? run(register, ["-u", app.path])
        try? files.removeItem(at: root)
    }
    try files.createDirectory(at: binary.deletingLastPathComponent(), withIntermediateDirectories: true)
    try files.createDirectory(at: resources, withIntermediateDirectories: true)
    try files.copyItem(at: source, to: resources.appendingPathComponent("Send to Gatto.workflow"))
    try files.copyItem(at: URL(fileURLWithPath: CommandLine.arguments[0]), to: binary)
    let info: [String: Any] = ["CFBundleExecutable": "receiver", "CFBundleIdentifier": identifier,
        "CFBundlePackageType": "APPL", "CFBundleName": "Gatto Finder Test", "LSUIElement": true,
        "CFBundleVersion": "1.0", "CFBundleInfoDictionaryVersion": "6.0",
        "GattoTestOutput": output.path]
    try PropertyListSerialization.data(fromPropertyList: info, format: .xml, options: 0)
        .write(to: contents.appendingPathComponent("Info.plist"))
    try run("/usr/bin/codesign", ["--force", "--sign", "-", app.path])
    try run("/usr/bin/codesign", ["--verify", "--deep", "--strict", app.path])
    try run(register, ["-f", app.path])
    try require(NSWorkspace.shared.urlForApplication(withBundleIdentifier: identifier) != nil,
        "Launch Services did not register the disposable app")

    // Load the production bundle with Automator before redirecting its sole
    // launch target to the fixture. All other metadata and parameters stay real.
    let workflow = try AMWorkflow(contentsOf: source)
    try require(workflow.actions.count == 1, "Expected one launch action")
    let action = workflow.actions[0] as! AMBundleAction
    let parameters = action.parameters!
    let script = parameters["COMMAND_STRING"] as! String
    try require(script.contains("com.jeremy-chandler.gatto"), "Missing app launch target")
    parameters["COMMAND_STRING"] = script.replacingOccurrences(of: "com.jeremy-chandler.gatto", with: identifier)
    let fixture = root.appendingPathComponent("Send to Fixture.workflow")
    try workflow.write(to: fixture)

    let paths = ["one space.png", "猫 #?%.mov", "line\nbreak.txt", "$(touch must-not-exist).png"]
        .map { root.appendingPathComponent($0).path }
    for path in paths { files.createFile(atPath: path, contents: Data()) }
    let expected = [[paths[0]], Array(paths.dropFirst())]
    for (index, selection) in expected.enumerated() {
        _ = try AMWorkflow.run(at: fixture, withInput: selection)
        let deadline = Date().addingTimeInterval(15)
        var received = [[String]]()
        while Date() < deadline {
            if let data = try? Data(contentsOf: output),
               let record = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let selections = record["selections"] as? [[String]] {
                received = selections
                if selections.count == index + 1 { break }
            }
            Thread.sleep(forTimeInterval: 0.05)
        }
        try require(received == Array(expected.prefix(index + 1)), "Cold/warm launch lost or split a selection: \(received)")
    }
    print("Finder workflow loaded; cold launch and existing-instance delivery preserved all paths and batches.")
}

if Bundle.main.bundleURL.pathExtension == "app" {
    let application = NSApplication.shared
    let receiver = Receiver()
    application.delegate = receiver
    application.setActivationPolicy(.accessory)
    DispatchQueue.main.asyncAfter(deadline: .now() + 45) { application.terminate(nil) }
    withExtendedLifetime(receiver) { application.run() }
} else {
    do { try verify() }
    catch { fputs("\(error)\n", stderr); exit(1) }
}
