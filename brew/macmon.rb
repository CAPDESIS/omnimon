class Macmon < Formula
  desc "Lightweight macOS system monitor with native process picker UI"
  homepage "https://github.com/chochy2001/macmon"
  url "https://github.com/chochy2001/macmon/releases/download/v2.1.1/macmon-2.1.1-macos-universal.tar.gz"
  sha256 "9e1893c91891479b8bd682ae9af1ffebecd6ea8b39a4aae652926a4bafee8486"
  license "MIT"
  version "2.1.1"

  depends_on "jq"
  depends_on :macos => :ventura

  def install
    # The tarball contains pre-compiled universal binaries (arm64 + x86_64)
    libexec.install "ProcessPicker"
    libexec.install "DiskIOHelper"
    libexec.install "MacmonStatusBar"
    libexec.install "lib"
    libexec.install "src"
    libexec.install "scripts"
    libexec.install "config"
    libexec.install "templates"

    # Localization resources
    resource_src = buildpath/"src/gui/Resources"
    if resource_src.exist?
      (libexec/"Resources").install resource_src.children
    end

    # CLI wrapper
    (bin/"macmon").write <<~EOS
      #!/usr/bin/env bash
      export MACMON_HOME="#{libexec}"
      export MACMON_CONFIG="${HOME}/.config/macmon/macmon.yaml"
      exec "#{libexec}/src/cli/macmon.sh" "$@"
    EOS
  end

  def post_install
    # Create user config directory with restrictive permissions
    config_dir = Pathname.new("#{ENV["HOME"]}/.config/macmon")
    config_dir.mkpath
    config_dir.chmod 0700

    profiles_dir = config_dir/"profiles"
    profiles_dir.mkpath
    profiles_dir.chmod 0700

    # Install default config if absent
    config_file = config_dir/"macmon.yaml"
    unless config_file.exist?
      cp libexec/"config/macmon.default.yaml", config_file
      config_file.chmod 0600
    end

    # Install default profiles if absent
    Dir[libexec/"config/profiles/*.yaml"].each do |profile|
      dest = profiles_dir/File.basename(profile)
      unless dest.exist?
        cp profile, dest
        dest.chmod 0600
      end
    end

    # Create log directory
    log_dir = Pathname.new("#{ENV["HOME"]}/.local/log/macmon")
    log_dir.mkpath
    log_dir.chmod 0700
  end

  service do
    run ["/bin/bash", opt_libexec/"src/daemon/macmond.sh"]
    keep_alive true
    process_type :background
    environment_variables MACMON_HOME: opt_libexec,
                          MACMON_CONFIG: "#{ENV["HOME"]}/.config/macmon/macmon.yaml"
    log_path "#{ENV["HOME"]}/.local/log/macmon/macmond.stdout.log"
    error_log_path "#{ENV["HOME"]}/.local/log/macmon/macmond.stderr.log"
  end

  def caveats
    <<~EOS
      macmon is installed. To get started:

        brew services start macmon    # start the background daemon
        macmon                        # open the native process picker
        macmon status                 # system health summary

      Menu bar monitor:
        MACMON_HOME="#{opt_libexec}" "#{opt_libexec}/MacmonStatusBar" &

      Configuration: ~/.config/macmon/macmon.yaml
      Logs:          ~/.local/log/macmon/macmond.log
    EOS
  end

  test do
    assert_match "macmon v", shell_output("#{bin}/macmon version")
  end
end
