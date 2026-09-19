# executable name, must match the package name in Cargo.toml
game := "bevy-starter"
exe := if os_family() == "windows" { ".exe" } else { "" }

# run the game in dev mode
run:
    cargo run

# apply clippy fixes and format the code
fix:
    cargo clippy --fix --allow-dirty
    cargo fmt

# check formatting, lints in dev and release configs, and run the tests
check:
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo clippy --no-default-features -- -D warnings
    cargo test

# build an optimized release and package it into dist/
release:
    cargo build --release --no-default-features
    rm -rf dist && mkdir -p dist
    cargo about generate --no-default-features --fail about.hbs -o dist/licenses.html
    cp target/release/{{game}}{{exe}} dist/
    cp -r assets dist/
