set shell := ["bash", "-euo", "pipefail", "-c"]

default:
  @just --list

help:
  @just --list

bootstrap-dev:
  @echo "Bootstrapping mini-conf development environment scaffold"
  @if [ ! -f Cargo.toml ]; then echo "Cargo workspace not initialized yet"; fi
  @if [ ! -f package.json ] && [ ! -f pnpm-workspace.yaml ] && [ ! -d apps/web ]; then echo "Frontend workspace not initialized yet"; fi

fmt:
  @just fmt-backend
  @just fmt-frontend

fmt-backend:
  @if [ -f Cargo.toml ]; then cargo fmt --all; else echo "Skipping backend fmt: Cargo.toml not found"; fi

fmt-frontend:
  @if [ -f package.json ] || [ -f pnpm-workspace.yaml ] || [ -f apps/web/package.json ]; then \
    if [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm exec prettier --write .; \
    else pnpm --dir apps/web exec prettier --write .; fi \
  ; else echo "Skipping frontend fmt: package manifest not found"; fi

lint:
  @just lint-backend
  @just lint-frontend

lint-backend:
  @if [ -f Cargo.toml ]; then cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings; else echo "Skipping backend lint: Cargo.toml not found"; fi

lint-frontend:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web lint && pnpm --dir apps/web format:check && pnpm --dir apps/web typecheck; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm lint && pnpm format:check && pnpm typecheck; \
  else echo "Skipping frontend lint: package manifest not found"; fi

test:
  @just test-backend
  @just test-frontend

test-backend:
  @if [ -f Cargo.toml ]; then cargo nextest run --workspace; else echo "Skipping backend tests: Cargo.toml not found"; fi

test-frontend:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web test; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test; \
  else echo "Skipping frontend tests: package manifest not found"; fi

test-e2e:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web test:e2e; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test:e2e; \
  else echo "Skipping e2e tests: package manifest not found"; fi

coverage:
  @if [ -f Cargo.toml ]; then cargo llvm-cov --workspace --lcov --output-path target/lcov.info; else echo "Skipping backend coverage: Cargo.toml not found"; fi

sqlx-check:
  @if [ -f Cargo.toml ]; then cargo sqlx prepare --check; else echo "Skipping sqlx prepare check: Cargo.toml not found"; fi

perf-smoke:
  @bash scripts/perf-smoke.sh

perf-ci:
  @PERF_ENFORCE=1 bash scripts/perf-smoke.sh

ci-local:
  @just lint
  @just test
  @just perf-smoke

dev-server:
  @if [ -f Cargo.toml ]; then cargo run --bin server; else echo "Skipping dev-server: Cargo workspace not initialized"; fi

dev-web:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web dev; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm dev; \
  else echo "Skipping dev-web: frontend workspace not initialized"; fi

db-reset-dev:
  @if [ -f scripts/db-reset-dev.sh ]; then bash scripts/db-reset-dev.sh; else echo "Skipping db reset: scripts/db-reset-dev.sh not found"; fi
