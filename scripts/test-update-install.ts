import { join, resolve } from "node:path";
import { homedir } from "node:os";
// Real Sparkle replacement/relaunch with disposable ad-hoc signed fixtures.
// Production Developer ID signing/notarization remains a release acceptance check.
const root = resolve(import.meta.dir, "..");
process.chdir(root);
await Bun.$`./scripts/prepare-updater.sh`.quiet();
const work = (await Bun.$`mktemp -d -t gatto-update-install`.quiet()).text().trim();
await Bun.$`mkdir -p ${work}/served ${work}/installed ${work}/new`;
const marker = join(work, "installed-version");
await Bun.$`rm -f ${marker}`;
const clicked = join(work, "clicked");
let feedRequests = 0;
let downloads = 0;
let downloadedBeforeClick = false;
const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  async fetch(req) {
    const name = new URL(req.url).pathname.split("/").pop()!;
    if (name === "appcast.xml") feedRequests += 1;
    if (name === "gatto-test.zip") {
      downloads += 1;
      if (!(await Bun.file(clicked).exists())) downloadedBeforeClick = true;
    }
    return new Response(Bun.file(join(work, "served", name)));
  },
});
const keyScript = join(work, "key.swift");
await Bun.write(
  keyScript,
  `import Foundation\nimport CryptoKit\nlet key = Curve25519.Signing.PrivateKey()\ntry key.rawRepresentation.base64EncodedString().write(toFile: CommandLine.arguments[1], atomically: true, encoding: .utf8)\nprint(key.publicKey.rawRepresentation.base64EncodedString())`,
);
const keyfile = join(work, "test-key");
const keys = await Bun.$`xcrun swift ${keyScript} ${keyfile}`.quiet();
await Bun.$`chmod 600 ${keyfile}`;
const publicKey = keys.text().trim();
const main = join(work, "Main.swift");
await Bun.write(
  main,
  `import AppKit
@main struct Harness {
 @MainActor static func main() {
  let app = NSApplication.shared
  app.setActivationPolicy(.prohibited)
  if Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String == "0.15.0" {
   try! "0.15.0".write(toFile: ${JSON.stringify(marker)}, atomically: true, encoding: .utf8)
   return
  }
  let recent = CommandLine.arguments.contains("--recent")
  let cached = CommandLine.arguments.contains("--cached")
  if cached { UserDefaults.standard.set("0.15.0", forKey: "GattoAvailableUpdate") }
  UserDefaults.standard.set(Date(timeIntervalSinceNow: (recent || cached) ? 0 : -5 * 60 * 60), forKey: "SULastCheckTime")
  let driver = UpdaterBridge()
  driver.start { state, message in
   NSLog("TEST state=%ld %@", state, message)
   if state == 1 {
    if recent { exit(21) }
    DispatchQueue.main.async {
     try! "clicked".write(toFile: ${JSON.stringify(clicked)}, atomically: true, encoding: .utf8)
     driver.install()
    }
   }
   if state == 3 || state == 4 { exit(20) }
  }
  if recent {
   DispatchQueue.main.asyncAfter(deadline: .now() + 1) { exit(0) }
  }
  withExtendedLifetime(driver) { app.run() }
 }
}`,
);
const exe = join(work, "harness");
await Bun.$`xcrun swiftc -target arm64-apple-macos13 -F dist/sparkle -framework Sparkle -Xlinker -rpath -Xlinker @executable_path/../Frameworks native/Updater.swift ${main} -o ${exe}`.quiet();
const id = `com.jeremy-chandler.gatto.acceptance${Date.now()}`;
let child: Bun.Subprocess<"ignore", "pipe", "pipe"> | undefined;
try {
  for (const { dir, version } of [
    { dir: "installed", version: "0.14.0" },
    { dir: "new", version: "0.15.0" },
  ]) {
    const app = join(work, dir, "Gatto.app");
    await Bun.$`mkdir -p ${app}/Contents/MacOS ${app}/Contents/Frameworks`;
    await Bun.$`cp ${exe} ${app}/Contents/MacOS/gatto`;
    await Bun.$`ditto dist/sparkle/Sparkle.framework ${app}/Contents/Frameworks/Sparkle.framework`;
    const plist = `<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>
 <key>CFBundleIdentifier</key><string>${id}</string><key>CFBundleName</key><string>Gatto updater test</string>
 <key>CFBundleExecutable</key><string>gatto</string><key>CFBundlePackageType</key><string>APPL</string>
 <key>CFBundleVersion</key><string>${version}</string><key>CFBundleShortVersionString</key><string>${version}</string>
 <key>LSMinimumSystemVersion</key><string>13.0</string><key>SUPublicEDKey</key><string>${publicKey}</string>
 <key>SUFeedURL</key><string>http://127.0.0.1:${server.port}/appcast.xml</string>
 <key>SUEnableAutomaticChecks</key><true/><key>SUScheduledCheckInterval</key><integer>14400</integer>
 <key>SUAllowsAutomaticUpdates</key><false/>
 <key>SURequireSignedFeed</key><true/><key>SUVerifyUpdateBeforeExtraction</key><true/>
 <key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>
 </dict></plist>`;
    await Bun.write(join(app, "Contents/Info.plist"), plist);
    await Bun.$`codesign --force --sign - ${app}`.quiet();
  }
  await Bun.$`ditto -c -k --keepParent ${work}/new/Gatto.app ${work}/served/gatto-test.zip`.quiet();
  await Bun.$`dist/sparkle/bin/generate_appcast --ed-key-file ${keyfile} --download-url-prefix http://127.0.0.1:${server.port}/ --maximum-deltas 0 ${work}/served`.quiet();
  const recent = Bun.spawn([join(work, "installed/Gatto.app/Contents/MacOS/gatto"), "--recent"], {
    stdin: "ignore",
    stdout: "ignore",
    stderr: "pipe",
  });
  const recentLog = new Response(recent.stderr).text();
  const recentTimeout = setTimeout(() => recent.kill(), 10000);
  try {
    if ((await recent.exited) !== 0) throw new Error(await recentLog);
    if (feedRequests !== 0 || downloads !== 0)
      throw new Error("Sparkle checked before the saved four-hour deadline");
  } finally {
    clearTimeout(recentTimeout);
  }
  await Bun.$`ditto ${work}/installed/Gatto.app ${work}/original.app`.quiet();
  for (const mode of ["--overdue", "--cached"]) {
    await Bun.$`rm -rf ${work}/installed/Gatto.app`;
    await Bun.$`ditto ${work}/original.app ${work}/installed/Gatto.app`.quiet();
    await Bun.$`rm -f ${marker} ${clicked}`;
    downloads = 0;
    feedRequests = 0;
    downloadedBeforeClick = false;
    child = Bun.spawn([join(work, "installed/Gatto.app/Contents/MacOS/gatto"), mode], {
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    const stderrPromise = new Response(child.stderr).text();
    const end = Date.now() + 20000;
    while (Date.now() < end && !(await Bun.file(marker).exists())) {
      await Bun.sleep(250);
    }
    if (!(await Bun.file(marker).exists())) {
      child.kill();
      console.log(await stderrPromise);
      throw new Error("No successful relaunch marker");
    }
    if (downloadedBeforeClick || Number(downloads) !== 1 || feedRequests < 1) {
      throw new Error("Expected exactly one archive download after the install click");
    }
    console.log(
      `Passed (${mode}): Sparkle honors a recent check; one click installs/relaunches ` +
        (await Bun.file(marker).text()),
    );
    await child.exited;
    await stderrPromise;
  }
} finally {
  child?.kill();
  server.stop();
  await Bun.$`defaults delete ${id}`.quiet().nothrow();
  await Bun.$`rm -rf ${work} ${join(homedir(), "Library/Caches", id)}`;
}
