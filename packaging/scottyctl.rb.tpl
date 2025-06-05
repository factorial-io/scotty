class Scottyctl < Formula
  desc "CLI tool for Scotty"
  homepage "https://github.com/factorial-io/scotty"
  version "VERSION_PLACEHOLDER"

  on_macos do
    if Hardware::CPU.arm?
      url "URL_MAC_ARM"
      sha256 "SHA_MAC_ARM"
    elsif Hardware::CPU.intel?
      url "URL_MAC_INTEL"
      sha256 "SHA_MAC_INTEL"
    end
  end

  on_linux do
    url "URL_LINUX"
    sha256 "SHA_LINUX"
  end

  def install
    bin.install "scottyctl"
  end

  test do
    system "#{bin}/scottyctl", "--help"
  end
end
