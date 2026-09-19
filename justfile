game := "bevy-starter"
exe := if os_family() == "windows" { ".exe" } else { "" }

run:
    cargo run

fix:
    cargo clippy --fix --allow-dirty
    cargo fmt

check:
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo clippy --no-default-features -- -D warnings

release:
    cargo build --release --no-default-features
    rm -rf dist && mkdir -p dist
    cargo about generate --no-default-features --fail about.hbs -o dist/licenses.html
    cp target/release/{{game}}{{exe}} dist/
    cp -r assets dist/
