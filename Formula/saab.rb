class Saab < Formula
  desc "Ultra-low-latency distributed audio streaming & mixing console CLI for macOS and Android"
  homepage "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming"
  version "0.4.0"
  license "MIT"

  if Hardware::CPU.arm?
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.0/saab-v0.4.0-macos-arm64.tar.gz"
    # sha256 checksum will be injected dynamically on release
  else
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.0/saab-v0.4.0-macos-x86_64.tar.gz"
    # sha256 checksum will be injected dynamically on release
  end

  depends_on "blackhole-16ch"

  def install
    bin.install "saab"
    bin.install "server" if File.exist?("server")
  end

  test do
    assert_match "saab #{version}", shell_output("#{bin}/saab --version")
  end
end
