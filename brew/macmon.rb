class Macmon < Formula
  desc "Lightweight macOS system monitor with native process picker UI"
  homepage "https://github.com/chochy2001/macmon"
  url "https://github.com/chochy2001/macmon/archive/refs/tags/v1.1.0.tar.gz"
  sha256 "" # Updated by release workflow
  license "MIT"
  head "https://github.com/chochy2001/macmon.git", branch: "main"

  depends_on "jq"
  depends_on :macos => :ventura

  def install
    # Compile Swift binaries
    system "swiftc", "-O", "-framework", "Cocoa",
           "-o", "ProcessPicker",
           "src/gui/ProcessPickerModel.swift",
           "src/gui/ProcessPicker.swift"
    system "swiftc", "-O",
           "-o", "DiskIOHelper",
           "src/gui/DiskIOHelper.swift"
    system "swiftc", "-O", "-framework", "Cocoa",
           "-o", "MacmonStatusBar",
           "src/gui/MacmonStatusBar.swift"

    # Install to libexec (not directly to bin)
    libexec.install "ProcessPicker"
    libexec.install "DiskIOHelper"
    libexec.install "MacmonStatusBar"
    libexec.install "lib"
    libexec.install "src"
    libexec.install "scripts"
    libexec.install "config"

    # Create wrapper script
    (bin/"macmon").write <<~EOS
      #!/usr/bin/env bash
      export MACMON_HOME="#{libexec}"
      export MACMON_CONFIG="${HOME}/.config/macmon/macmon.yaml"
      exec "#{libexec}/src/cli/macmon.sh" "$@"
    EOS

    # Install default config
    etc.install "config/macmon.default.yaml" => "macmon/macmon.default.yaml"

    # Install LaunchAgent plist template
    prefix.install "templates/com.macmon.daemon.plist.in"
  end

  def post_install
    # Create user config if absent
    config_dir = Pathname.new("#{ENV["HOME"]}/.config/macmon")
    config_dir.mkpath
    config_file = config_dir/"macmon.yaml"
    unless config_file.exist?
      cp etc/"macmon/macmon.default.yaml", config_file
    end

    # Create log directory
    log_dir = Pathname.new("#{ENV["HOME"]}/.local/log/macmon")
    log_dir.mkpath
  end

  def caveats
    <<~EOS
      To start the macmon daemon:
        macmon start

      To install the LaunchAgent for auto-start on login:
        cp #{prefix}/com.macmon.daemon.plist.in ~/Library/LaunchAgents/com.macmon.daemon.plist
        # Edit the plist to replace @@PLACEHOLDERS@@
        launchctl load -w ~/Library/LaunchAgents/com.macmon.daemon.plist

      To launch the menu bar monitor:
        #{libexec}/MacmonStatusBar &

      Configuration: ~/.config/macmon/macmon.yaml
      Logs: ~/.local/log/macmon/macmond.log
    EOS
  end

  test do
    assert_match "macmon v", shell_output("#{bin}/macmon version")
    system "#{libexec}/DiskIOHelper", "1"
  end
end
