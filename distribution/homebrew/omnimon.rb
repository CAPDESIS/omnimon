cask "omnimon" do
  version "4.0.6"
  sha256 "2647ced97ce92aca663a6c0ca10c6529398640924bc0899da56b788d37ccde2b"

  url "https://github.com/chochy2001/macmon/releases/download/v#{version}/macmon_#{version}_aarch64.dmg"
  name "OmniMon"
  desc "Cross-platform system monitor, process manager, and AI assistant"
  homepage "https://github.com/chochy2001/macmon"

  app "macmon.app"

  zap trash: [
    "~/Library/Application Support/com.omnimon.app",
    "~/Library/Caches/com.omnimon.app",
    "~/Library/Preferences/com.omnimon.app.plist",
    "~/.config/macmon",
    "~/.local/share/macmon"
  ]
end