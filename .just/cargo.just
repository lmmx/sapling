build:
    cargo build --workspace --all-features

check:
    # cargo check --workspace --all-features
    # cargo hack check --workspace --each-feature --no-dev-deps
    cargo hack check --workspace --feature-powerset --no-dev-deps

clippy:
    cargo clippy --workspace --all-targets --all-features --target-dir target/clippy-all-features -- -D warnings

clippy-fix:
    cargo clippy --fix --workspace --all-targets --all-features --target-dir target/clippy-all-features -- -D warnings

docs:
    cargo doc --all-features --no-deps --document-private-items --keep-going

ndocs:
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features --open
    rustup toolchain uninstall nightly

doctest *args:
    cargo test --doc {{args}} --all-features

test *args:
    # cargo nextest run {{args}} --all-targets --all-features --no-fail-fast
    # cargo hack nextest run --workspace --all-targets --each-feature --no-fail-fast
    cargo hack nextest run --workspace --all-targets --feature-powerset --no-fail-fast

test-ci *args:
    #!/usr/bin/env -S echo-comment --color bright-green
    # 🏃 Running Rust tests...
    cargo test {{args}}

    # 📚 Running documentation tests...
    cargo test --doc {{args}}
