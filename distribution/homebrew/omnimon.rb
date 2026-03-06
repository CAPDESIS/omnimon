cask "omnimon" do
  version "4.2.0"
  sha256 "67d8e65a92d0b7f6fa6051b9ca6fb3c320725d1a8bff57e64e986e65f5bcbe4f"

  url "https://github.com/chochy2001/omnimon/releases/download/v#{version}/OmniMon_#{version}_aarch64.dmg"
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
