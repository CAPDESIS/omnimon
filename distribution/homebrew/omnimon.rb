cask "omnimon" do
  version "6.6.6"
  sha256 "8293a3f22f507b1144cb4492197176c0db4c537aeacf314c7521984fd0a289f2"

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
