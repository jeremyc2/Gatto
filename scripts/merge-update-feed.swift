// Merge already signature-verified release feeds. Re-sign the output afterwards.
import Foundation

let sparkle = "http://www.andymatuschak.org/xml-namespaces/sparkle"
struct Version: Comparable {
    let core: [UInt64]
    let beta: UInt64?
    init(_ text: String) throws {
        guard text.range(of: #"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-beta\.([1-9]\d*))?$"#, options: .regularExpression) != nil else {
            throw NSError(domain: "GattoFeed", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid release version: \(text)"])
        }
        let parts = text.components(separatedBy: "-beta.")
        core = try parts[0].split(separator: ".").map {
            guard let value = UInt64($0) else { throw CocoaError(.coderInvalidValue) }
            return value
        }
        if parts.count == 2 {
            guard let value = UInt64(parts[1]) else { throw CocoaError(.coderInvalidValue) }
            beta = value
        } else { beta = nil }
    }
    static func < (a: Version, b: Version) -> Bool {
        if a.core != b.core { return a.core.lexicographicallyPrecedes(b.core) }
        if a.beta == nil { return false }
        if b.beta == nil { return true }
        return a.beta! < b.beta!
    }
}
func main() throws {
    let args = Array(CommandLine.arguments.dropFirst())
    guard args.count >= 2 else { throw CocoaError(.fileReadInvalidFileName) }
    var newest: [String: (Version, XMLElement)] = [:]
    for path in args.dropFirst() {
        let data = try Data(contentsOf: URL(fileURLWithPath: path))
        guard data.count <= 2 * 1024 * 1024,
              !String(decoding: data, as: UTF8.self).contains("<!DOCTYPE") else { throw CocoaError(.coderInvalidValue) }
        let document = try XMLDocument(data: data, options: [.nodeLoadExternalEntitiesNever])
        guard document.rootElement()?.name == "rss",
              let channel = document.rootElement()?.elements(forName: "channel").first else { throw CocoaError(.coderInvalidValue) }
        for item in channel.elements(forName: "item") {
            guard let build = item.elements(forLocalName: "version", uri: sparkle).first?.stringValue,
                  let text = item.elements(forLocalName: "shortVersionString", uri: sparkle).first?.stringValue,
                  build == text.replacingOccurrences(of: "-beta.", with: "b") else { throw CocoaError(.coderInvalidValue) }
            let version = try Version(text)
            let releaseChannel = item.elements(forLocalName: "channel", uri: sparkle).first?.stringValue
            guard releaseChannel == (version.beta == nil ? nil : "beta"),
                  let enclosure = item.elements(forName: "enclosure").first,
                  let url = enclosure.attribute(forName: "url")?.stringValue,
                  url == "https://github.com/jeremyc2/Gatto/releases/download/v\(text)/gatto-\(text)-aarch64-apple-darwin.dmg",
                  enclosure.attribute(forLocalName: "edSignature", uri: sparkle) != nil else { throw CocoaError(.coderInvalidValue) }
            let key = releaseChannel ?? "stable"
            if newest[key] == nil || newest[key]!.0 <= version {
                newest[key] = (version, item.copy() as! XMLElement)
            }
        }
    }
    guard newest["stable"] != nil else { throw CocoaError(.coderInvalidValue) }
    let root = XMLElement(name: "rss")
    root.addAttribute(XMLNode.attribute(withName: "version", stringValue: "2.0") as! XMLNode)
    root.addNamespace(XMLNode.namespace(withName: "sparkle", stringValue: sparkle) as! XMLNode)
    let channel = XMLElement(name: "channel")
    channel.addChild(XMLElement(name: "title", stringValue: "Gatto Beta and Stable"))
    for (_, item) in newest.values.sorted(by: { $0.0 > $1.0 }) { channel.addChild(item) }
    root.addChild(channel)
    try XMLDocument(rootElement: root).xmlData(options: .nodePrettyPrint).write(to: URL(fileURLWithPath: args[0]), options: .atomic)

}
do { try main() }
catch {
    fputs("Cannot merge release feeds: \(error.localizedDescription)\n", stderr)
    exit(1)
}
