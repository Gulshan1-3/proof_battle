.PHONY: bootstrap verify-harness run-server env-report fmt clippy test

bootstrap:
	@cargo check --manifest-path server/Cargo.toml

verify-harness:
	@bash scripts/verify_corpus.sh

run-server:
	@cargo run --manifest-path server/Cargo.toml

env-report:
	@bash scripts/env_report.sh

fmt:
	@cargo fmt --check --manifest-path server/Cargo.toml

clippy:
	@cargo clippy --manifest-path server/Cargo.toml -- -D warnings

test:
	@cargo test --manifest-path server/Cargo.toml
