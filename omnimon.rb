class Omnimon < Formula
  desc "OmniMon: The ultimate monitoring tool"
  homepage "https://github.com/omnimon/omnimon"
  url "https://github.com/omnimon/omnimon/archive/refs/tags/v6.0.1.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"

  def install
    bin.install "omnimon"
  end

  test do
    system "#{bin}/omnimon", "--version"
  end
end