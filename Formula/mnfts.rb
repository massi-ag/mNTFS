class Mnfts < Formula
  desc "Free, open-source NTFS driver for macOS (read-only)"
  homepage "https://github.com/maitgherbi/mnfts"
  url "https://github.com/maitgherbi/mnfts/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on :macos
  depends_on xcode: ["16.0", :build]

  def install
    system "cargo", "build", "--release", "--manifest-path", "Cargo.toml"
    cd "swift" do
      system "swift", "build", "-c", "release"
    end
    bin.install "swift/.build/release/mnfts"
  end

  def post_install
    ohai "Run 'mnfts mount /dev/diskNsN' to mount an NTFS volume"
  end

  test do
    assert_match "mnfts 0.1.0", shell_output("#{bin}/mnfts --version")
  end
end
