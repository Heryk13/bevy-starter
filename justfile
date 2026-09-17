run:
    cargo run

fix:
    cargo clippy --fix --allow-dirty
    cargo fmt

release:
    cargo build --release --no-default-features
