game := "bevy-starter"
exe := if os_family() == "windows" { ".exe" } else { "" }

run:
    cargo run

fix:
    cargo clippy --fix --allow-dirty
    cargo fmt

release:
    cargo build --release --no-default-features
    rm -rf dist && mkdir -p dist
    cp target/release/{{game}}{{exe}} dist/
    cp -r assets dist/
