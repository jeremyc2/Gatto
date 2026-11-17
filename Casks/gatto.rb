cask "gatto" do
  version "0.24.7"
  sha256 "4ae9751d9214ea6ab41d03fa2ea2c40aadf1ed3a3fbbded6675b14e26d783858"

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
