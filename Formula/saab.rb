class Saab < Formula
  desc "Ultra-low-latency distributed audio mixing console CLI for macOS and Android"
  homepage "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming"
  version "0.4.1"
  license "MIT"

  if Hardware::CPU.arm?
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.1/saab-v0.4.1-macos-arm64.tar.gz"
    sha256 "e079daded459a4908cf084bddc43751edff423bc5c6c138d11caf56f349c4651"
  else
    url "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v0.4.1/saab-v0.4.1-macos-x86_64.tar.gz"
    sha256 "030597975027ed29b619546182038b50a2cada4e9c5661ad947987b5d72d7cec"
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
