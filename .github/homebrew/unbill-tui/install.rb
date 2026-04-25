bin.install "unbill-tui-macos-aarch64" => "unbill-tui" if Hardware::CPU.arm?
bin.install "unbill-tui-linux-x86_64" => "unbill-tui" if OS.linux?
