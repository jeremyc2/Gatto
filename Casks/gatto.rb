cask "gatto" do
  version "0.20.6"
  sha256 "0a137bfac30ab66129d7514c18771e0327631c7e05bb7185558c4c6d0903a589"

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
