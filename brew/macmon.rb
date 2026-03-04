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

    # CLI wrapper that initializes config on first run
    (bin/"macmon").write <<~EOS
      #!/usr/bin/env bash
      export MACMON_HOME="#{libexec}"
      _cfg="${HOME}/.config/macmon"
      if [[ ! -d "$_cfg" ]]; then
        mkdir -p "$_cfg/profiles" && chmod 700 "$_cfg" "$_cfg/profiles"
        cp "#{libexec}/config/macmon.default.yaml" "$_cfg/macmon.yaml" 2>/dev/null && chmod 600 "$_cfg/macmon.yaml"
        for p in "#{libexec}"/config/profiles/*.yaml; do
          [[ -f "$p" ]] && cp "$p" "$_cfg/profiles/" && chmod 600 "$_cfg/profiles/$(basename "$p")"
        done
      fi
      _log="${HOME}/.local/log/macmon"
      [[ -d "$_log" ]] || mkdir -p "$_log" && chmod 700 "$_log"
      export MACMON_CONFIG="$_cfg/macmon.yaml"
      exec "#{libexec}/src/cli/macmon.sh" "$@"
    EOS
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

        brew services start chochy2001/tap/macmon   # start the background daemon
        macmon                                      # open the native process picker
        macmon status                               # system health summary

      Menu bar monitor:
        MACMON_HOME="#{opt_libexec}" "#{opt_libexec}/MacmonStatusBar" &

      Configuration: ~/.config/macmon/macmon.yaml
      Logs:          ~/.local/log/macmon/macmond.log

      Config and log directories are created automatically on first run.
    EOS
  end

  test do
    assert_match "macmon v", shell_output("#{bin}/macmon version")
  end
end
