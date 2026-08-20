class Saab < Formula
  desc "Ultra-low-latency distributed audio mixing console CLI for macOS and Android"
  homepage "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming"
  version "0.4.0"
  license "MIT"

  if Hardware::CPU.arm?
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.0/saab-v0.4.0-macos-arm64.tar.gz"
    sha256 "e7d2331c9af44d3c6220e3cffa7a7437bbd55a4ffc0ee175b12dd9d7ce5d865d"
  else
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.0/saab-v0.4.0-macos-x86_64.tar.gz"
    sha256 "d59e0f8368ea905973a6d279e5401744ad3fd71b8e253651f3984fc73213f68f"
  end

  def install
    bin.install "saab"
    bin.install "server" if File.exist?("server")
  end

  def caveats
    <<~EOS
      Saab captures macOS system audio via CoreAudio HAL loopback drivers.
      If you do not have BlackHole 16ch installed yet, run:
        brew install --cask blackhole-16ch
    EOS
  end

  test do
    assert_match "saab #{version}", shell_output("#{bin}/saab --version")
  end
end
