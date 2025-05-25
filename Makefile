test:
	cargo test --verbose -- --nocapture --test-threads=1

format:
	cargo fmt --all -- --check

lint:
	cargo clippy --verbose -- -D warnings

build:
	cargo run --release