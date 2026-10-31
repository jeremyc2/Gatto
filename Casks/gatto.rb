cask "gatto" do
  version "0.22.0"
  sha256 "b6d19ea60199af9c1fc75e11b80d624e483397e9b7e6195af703f4d8000b7421"

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
