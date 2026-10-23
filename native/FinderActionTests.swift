// Verify the shipped document declarations and file delivery with a disposable app.
// Never registers a real Gatto bundle or installs a user service.
import AppKit
import CoreServices
import Foundation

@_silgen_name("GattoInstallFileService")
func installFileService()

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
    _ = NSApplication.shared
    let files = FileManager.default
    let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        .appendingPathComponent("dist/gatto-finder-\(UUID().uuidString)")
    try files.createDirectory(at: root, withIntermediateDirectories: true)
    let app = root.appendingPathComponent("Receiver.app")
    let contents = app.appendingPathComponent("Contents")
    let binary = contents.appendingPathComponent("MacOS/receiver")
    let source = URL(fileURLWithPath: CommandLine.arguments[1])
    let output = root.appendingPathComponent("received.json")
    let identifier = "com.jeremy-chandler.gatto.finder-test.\(UUID().uuidString)"
    let serviceName = "Send to Gatto Test \(UUID().uuidString)"
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
    try files.copyItem(at: URL(fileURLWithPath: CommandLine.arguments[0]), to: binary)
    var info = try PropertyListSerialization.propertyList(from: Data(contentsOf: source), options: [], format: nil) as! [String: Any]
    let declarations = info["CFBundleDocumentTypes"] as! [[String: Any]]
    let supported = ["public.png", "public.jpeg", "com.compuserve.gif", "org.webmproject.webp",
        "public.mpeg-4", "com.apple.quicktime-movie", "org.webmproject.webm"]
    try require(declarations.count == 1, "Expected one supported-media declaration")
    try require(declarations[0]["CFBundleTypeRole"] as? String == "Viewer", "Expected a viewer")
    try require(declarations[0]["LSHandlerRank"] as? String == "Alternate", "Must preserve default apps")
    try require(declarations[0]["LSItemContentTypes"] as? [String] == supported, "Unexpected supported types")
    info["CFBundleExecutable"] = "receiver"
    info["CFBundleIdentifier"] = identifier
    info["CFBundleName"] = "Gatto Finder Test"
    info["LSUIElement"] = true
    info["GattoTestOutput"] = output.path
    info.removeValue(forKey: "CFBundleURLTypes")
    var services = info["NSServices"] as! [[String: Any]]
    try require(services.count == 1 && services[0]["NSMessage"] as? String == "sendToGatto",
        "Missing native file service")
    try require(services[0]["NSSendFileTypes"] as? [String] == ["public.item"],
        "Service must preserve mixed selections")
    // The test client is not Finder; keep production filtering out of this fixture.
    services[0]["NSRequiredContext"] = [String: String]()
    services[0]["NSMenuItem"] = ["default": serviceName]
    services[0]["NSPortName"] = "Gatto Finder Test"
    info["NSServices"] = services
    try PropertyListSerialization.data(fromPropertyList: info, format: .xml, options: 0)
        .write(to: contents.appendingPathComponent("Info.plist"))
    try run("/usr/bin/codesign", ["--force", "--sign", "-", app.path])
    try run("/usr/bin/codesign", ["--verify", "--deep", "--strict", app.path])
    try run(register, ["-f", app.path])
    try require(NSWorkspace.shared.urlForApplication(withBundleIdentifier: identifier) != nil,
        "Launch Services did not register the disposable app")

    for type in supported {
        let handlers = LSCopyAllRoleHandlersForContentType(type as CFString, .viewer)?.takeRetainedValue() as? [String] ?? []
        try require(handlers.contains { $0.caseInsensitiveCompare(identifier) == .orderedSame },
            "Open With registration missing for \(type)")
    }

    let paths = ["one space.png", "猫 #?%.mov", "line\nbreak.txt", "$(touch must-not-exist).png"]
        .map { root.appendingPathComponent($0).path }
    for path in paths { files.createFile(atPath: path, contents: Data()) }
    let expected = [[paths[0]], Array(paths.dropFirst())]
    for (index, selection) in expected.enumerated() {
        try run("/usr/bin/open", ["-b", identifier, "--"] + selection)
        let deadline = Date().addingTimeInterval(15)
        var received = [[String]]()
        while Date() < deadline {
            if let data = try? Data(contentsOf: output),
               let record = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let selections = record["selections"] as? [[String]] {
                received = selections
                if selections.count == index + 1 { break }
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        try require(received == Array(expected.prefix(index + 1)), "Cold/warm launch lost or split a selection: \(received)")
    }
    // Exercise the production service through AppKit, including a fresh launch.
    for app in NSRunningApplication.runningApplications(withBundleIdentifier: identifier) {
        app.terminate()
        let deadline = Date().addingTimeInterval(10)
        while !app.isTerminated && Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.05)) }
        try require(app.isTerminated, "Receiver did not quit before cold service test")
    }
    try files.removeItem(at: output)
    NSUpdateDynamicServices()
    RunLoop.current.run(until: Date().addingTimeInterval(1))
    for (index, selection) in expected.enumerated() {
        let board = NSPasteboard.withUniqueName()
        defer { board.releaseGlobally() }
        board.writeObjects(selection.map { NSURL(fileURLWithPath: $0) })
        try require(NSPerformService(serviceName, board), "Native service dispatch failed")
        let deadline = Date().addingTimeInterval(15)
        var received = [[String]]()
        while Date() < deadline {
            if let data = try? Data(contentsOf: output),
               let record = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let selections = record["selections"] as? [[String]] {
                received = selections
                if selections.count == index + 1 { break }
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        try require(received == Array(expected.prefix(index + 1)), "Service lost or split selection: \(received)")
    }
    print("Open With and native service passed cold/warm delivery with intact paths and batches.")
}

if Bundle.main.bundleURL.pathExtension == "app" {
    let application = NSApplication.shared
    let receiver = Receiver()
    application.delegate = receiver
    application.setActivationPolicy(.accessory)
    installFileService()
    DispatchQueue.main.asyncAfter(deadline: .now() + 45) { application.terminate(nil) }
    withExtendedLifetime(receiver) { application.run() }
} else {
    do { try verify() }
    catch { fputs("\(error)\n", stderr); exit(1) }
}
