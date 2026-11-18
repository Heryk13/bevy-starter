# executable name, must match the package name in Cargo.toml
game := "bevy-starter"
exe := if os_family() == "windows" { ".exe" } else { "" }

# run the game in dev mode
run:
    cargo run

# run the game recording how long every system takes, into a dated file in profiles/ written
# when the game closes. To read it, open https://ui.perfetto.dev in a browser and drag the
# file in: nothing to install. Traced builds are slower, so compare systems with each other,
# not with a release build, and keep runs short: the file grows by hundreds of MB a minute
profile:
    mkdir -p profiles
    file="profiles/trace-$(date +%Y-%m-%d-%H%M%S).json"; TRACE_CHROME="$file" cargo run --features bevy/trace_chrome && echo "profile written to $file: open https://ui.perfetto.dev and drag it in"

# apply clippy fixes and format the code
fix:
    cargo clippy --all-targets --fix --allow-dirty
    cargo fmt

# check formatting, lints (tests included) in dev and release configs, and run the tests
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --no-default-features -- -D warnings
    cargo test

# run the tests only
test:
    cargo test

# build an optimized release and package it into dist/
release:
    cargo build --release --no-default-features
    rm -rf dist && mkdir -p dist
    cargo about generate --no-default-features --fail about.hbs -o dist/licenses.html
    cp target/release/{{game}}{{exe}} dist/
    cp -r assets dist/
