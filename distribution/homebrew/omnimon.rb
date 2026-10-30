# Tracks the latest published GitHub release (v6.8.0 Universal DMG,
# 2026-10-26). Bump version and sha256 together, only from a published DMG.
cask "omnimon" do
  version "6.8.0"
  sha256 "1d2f3cd9f96b9f2f9ebd32c49a08951b6ad1aa0cb6e745f6a88ee3b7d249b03c"

  url "https://github.com/chochy2001/omnimon/releases/download/v#{version}/OmniMon-#{version}-macOS-Universal.dmg"
  name "OmniMon"
  desc "Cross-platform system monitor, process manager, and AI assistant"
  homepage "https://github.com/chochy2001/omnimon"

  depends_on macos: ">= :ventura"

  app "OmniMon.app"

  postflight do
    ohai "Launching OmniMon..."
    system "open", "/Applications/OmniMon.app"
  end

  uninstall quit: "com.omnimon.desktop"

  zap trash: [
    "~/Library/Application Support/com.omnimon.desktop",
    "~/Library/Caches/com.omnimon.desktop",
    "~/Library/Preferences/com.omnimon.desktop.plist",
    "~/.config/macmon",
    "~/.local/share/macmon"
  ]

  caveats <<~EOS
    OmniMon is now in your Applications folder.

    To open:  ⌘ + Space → type "OmniMon"
    Or run:   open /Applications/OmniMon.app

    OmniMon also runs in your menu bar tray.
  EOS
end
