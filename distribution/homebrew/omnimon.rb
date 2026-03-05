cask "omnimon" do
  version "4.0.4"
  # TODO: Reemplaza con el SHA-256 real del .dmg (shasum -a 256 OmniMon_4.0.4_x64.dmg)
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/chochy2001/macmon/releases/download/v#{version}/OmniMon_#{version}_x64.dmg"
  name "OmniMon"
  desc "Cross-platform system monitor, process manager, and AI assistant"
  homepage "https://github.com/chochy2001/macmon"

  app "OmniMon.app"

  zap trash: [
    "~/Library/Application Support/com.omnimon.app",
    "~/Library/Caches/com.omnimon.app",
    "~/Library/Preferences/com.omnimon.app.plist",
    "~/.config/macmon",
    "~/.local/share/macmon"
  ]
end