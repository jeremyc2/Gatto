cask "gatto" do
  version "0.24.8"
  sha256 "45459c80289dc00e0f3adeb17a05bc6983427ced3b3d32764da5fbd77de46c3e"

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
