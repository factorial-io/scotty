class Scottyctl < Formula
  desc "CLI tool for Scotty"
  homepage "https://github.com/factorial-io/scotty"
  version "VERSION_PLACEHOLDER"

  head do
    url "https://github.com/factorial-io/scotty.git", branch: "next"
  end

  depends_on "rust" => :build

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
    if build.head?
      system "cargo", "install", "--path", "scottyctl", "--bin", "scottyctl", "--root", prefix
    else
      bin.install "scottyctl"
    end
  end

  test do
    system "#{bin}/scottyctl", "--help"
  end
end
