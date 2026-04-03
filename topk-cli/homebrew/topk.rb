class Topk < Formula
  desc "Ingest files into datasets, then search or ask questions with cited answers"
  homepage "https://topk.io"
  version "__VERSION__"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/topk-io/topk/releases/download/cli-v#{version}/topk-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_AARCH64_APPLE_DARWIN__"
    else
      url "https://github.com/topk-io/topk/releases/download/cli-v#{version}/topk-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_X86_64_APPLE_DARWIN__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/topk-io/topk/releases/download/cli-v#{version}/topk-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_AARCH64_LINUX__"
    else
      url "https://github.com/topk-io/topk/releases/download/cli-v#{version}/topk-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_X86_64_LINUX__"
    end
  end

  def install
    bin.install "topk"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/topk --version")
  end
end
