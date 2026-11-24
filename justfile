# executable name, must match the package name in Cargo.toml
game := "bevy-starter"

# PowerShell on Windows, which every Windows has: just's default there is `sh`, which only a Git
# for Windows that put its Unix tools on PATH brings. A recipe the two shells write differently
# comes twice, `[windows]` and `[unix]`
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# run the game in dev mode
run:
    cargo run

# run the game recording how long every system takes, into a dated file in profiles/ written
# when the game closes. To read it, open https://ui.perfetto.dev in a browser and drag the
# file in: nothing to install. Traced builds are slower, so compare systems with each other,
# not with a release build, and keep runs short: the file grows by hundreds of MB a minute
[unix]
profile:
    mkdir -p profiles
    file="profiles/trace-$(date +%Y-%m-%d-%H%M%S).json"; TRACE_CHROME="$file" cargo run --features bevy/trace_chrome && echo "profile written to $file: open https://ui.perfetto.dev and drag it in"

# run the game recording how long every system takes, into a dated file in profiles/ written
# when the game closes. To read it, open https://ui.perfetto.dev in a browser and drag the
# file in: nothing to install. Traced builds are slower, so compare systems with each other,
# not with a release build, and keep runs short: the file grows by hundreds of MB a minute
[windows]
profile:
    New-Item -ItemType Directory -Force profiles | Out-Null
    $file = "profiles/trace-$(Get-Date -Format yyyy-MM-dd-HHmmss).json"; $env:TRACE_CHROME = $file; cargo run --features bevy/trace_chrome; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; "profile written to ${file}: open https://ui.perfetto.dev and drag it in"

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
[unix]
release:
    cargo build --release --no-default-features
    rm -rf dist && mkdir -p dist
    cargo about generate --no-default-features --fail about.hbs -o dist/licenses.html
    cp target/release/{{game}} dist/
    cp -r assets dist/

# build an optimized release and package it into dist/
[windows]
release:
    cargo build --release --no-default-features
    if (Test-Path dist) { Remove-Item -Recurse -Force dist }; New-Item -ItemType Directory dist | Out-Null
    cargo about generate --no-default-features --fail about.hbs -o dist/licenses.html
    Copy-Item target/release/{{game}}.exe dist/
    Copy-Item -Recurse assets dist/
