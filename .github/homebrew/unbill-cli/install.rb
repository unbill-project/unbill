bin.install "unbill-cli-macos-aarch64" => "unbill-cli" if Hardware::CPU.arm?
bin.install "unbill-cli-linux-x86_64" => "unbill-cli" if OS.linux?
