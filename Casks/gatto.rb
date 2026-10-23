cask "gatto" do
  version "0.17.3"
  sha256 "85d2402dd2e3a0a187a239c9002fce4d29054f8f3a98b639125250a130c94779"

  url "https://github.com/jeremyc2/Gatto/releases/download/v#{version}/gatto-#{version}-aarch64-apple-darwin.dmg"
  name "Gatto"
  desc "A macOS app for adding images to GitHub"
  homepage "https://github.com/jeremyc2/Gatto"

  auto_updates true

  depends_on arch: :arm64
  depends_on formula: "gh"
  depends_on macos: :ventura

  app "Gatto.app"

  uninstall launchctl: "com.jeremy-chandler.gatto",
            quit:      "com.jeremy-chandler.gatto"

  zap trash: [
    "~/Library/Application Support/Gatto",
    "~/Library/Caches/com.jeremy-chandler.gatto",
    "~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist",
    "~/Library/Preferences/com.jeremy-chandler.gatto.plist",
  ]
end
