.PHONY: bootstrap verify-harness run-server env-report fmt clippy test codegen db-up db-down db-reset migrate seed

bootstrap:
	@cargo check --manifest-path server/Cargo.toml

codegen:
	@cargo test --manifest-path server/Cargo.toml --test protocol_contract export_bindings -- --nocapture
	@git diff --quiet frontend/src/lib/ws/generated.ts || echo "TypeScript bindings updated."

verify-harness:
	@bash scripts/verify_judge.sh

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

db-up:
	docker compose -f infra/docker-compose.yml up -d

db-down:
	docker compose -f infra/docker-compose.yml down

db-reset:
	docker compose -f infra/docker-compose.yml down -v
	docker compose -f infra/docker-compose.yml up -d

migrate:
	DATABASE_URL="postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle" \
	  cargo run --manifest-path server/Cargo.toml --bin verify_problems -- --migrate-only

seed:
	DATABASE_URL="postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle" \
	  TARGET_VERIFIED=120 VERIFY_CONCURRENCY=8 \
	  cargo run --manifest-path server/Cargo.toml --bin verify_problems

