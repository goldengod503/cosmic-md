export PATH := env('HOME') + "/.cargo/bin:" + env('PATH')

# Build release binary
build:
    cargo build --release

# Run with a markdown file
run FILE:
    cargo run --release -- {{FILE}}

# Install binary + desktop entry, register MIME type
install: build
    mkdir -p ~/.local/bin
    cp target/release/galaxy-md ~/.local/bin/
    mkdir -p ~/.local/share/applications
    sed 's|Exec=galaxy-md|Exec={{env('HOME')}}/.local/bin/galaxy-md|' res/com.galaxy.md-viewer.desktop > ~/.local/share/applications/com.galaxy.md-viewer.desktop
    mkdir -p ~/.local/share/mime/packages
    cp res/text-markdown.xml ~/.local/share/mime/packages/
    update-mime-database ~/.local/share/mime
    update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
    xdg-mime default com.galaxy.md-viewer.desktop text/markdown
    xdg-mime default com.galaxy.md-viewer.desktop text/x-markdown
    @echo "Installed galaxy-md"

# Uninstall
uninstall:
    rm -f ~/.local/bin/galaxy-md
    rm -f ~/.local/share/applications/com.galaxy.md-viewer.desktop
    rm -f ~/.local/share/mime/packages/text-markdown.xml
    update-mime-database ~/.local/share/mime
    update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
    @echo "Uninstalled galaxy-md"

# Clean build artifacts
clean:
    cargo clean
