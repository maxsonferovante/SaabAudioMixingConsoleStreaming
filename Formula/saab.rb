class Saab < Formula
  desc "Ultra-low-latency distributed audio mixing console CLI for macOS and Android"
  homepage "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming"
  version "0.4.3"
  license "MIT"

  if Hardware::CPU.arm?
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.3/saab-v0.4.3-macos-arm64.tar.gz"
    sha256 "60268e3a41e2e30223b75cb700b0d686b9e73bbfd8aa0704df2fa7916762988a"
  else
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.3/saab-v0.4.3-macos-x86_64.tar.gz"
    sha256 "8f71e048ed128efc4f569777434f0ab47b1c603b3c5abcdbaea8962bb74f1efa"
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
