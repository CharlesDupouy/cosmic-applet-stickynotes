name   := 'cosmic-applet-stickynotes'
appid  := 'com.github.CharlesDupouy.CosmicAppletStickynotes'

rootdir := ''
prefix  := env('PREFIX', '/usr')

# Derived installation paths
base-dir     := absolute_path(clean(rootdir / prefix))
cargo-target := env('CARGO_TARGET_DIR', 'target')
bin-dst      := base-dir / 'bin' / name
desktop-dst  := base-dir / 'share' / 'applications' / appid + '.desktop'
icon-dst     := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'
metainfo-dst := base-dir / 'share' / 'metainfo' / appid + '.metainfo.xml'

# Default recipe: build in release mode.
default: build-release

# Clean build artifacts.
clean:
    cargo clean

# Debug build.
build-debug *args:
    cargo build {{args}}

# Release build (default).
build-release *args: (build-debug '--release' args)

# Build and run in debug mode (for development).
run: build-debug
    cargo run

# Install into the system prefix (run build-release first as regular user).
install:
    install -Dm0755 {{cargo-target}}/release/{{name}}           {{bin-dst}}
    install -Dm0644 res/icons/hicolor/scalable/apps/{{appid}}.svg {{icon-dst}}
    install -Dm0644 res/{{appid}}.metainfo.xml                   {{metainfo-dst}}
    install -Dm0644 res/{{appid}}.desktop                        {{desktop-dst}}

# Uninstall from system prefix.
uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}} {{metainfo-dst}}
