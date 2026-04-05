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

test-backend-db:
  @if [ -f Cargo.toml ]; then source scripts/dev-db-env.sh && cargo nextest run --workspace; else echo "Skipping backend db tests: Cargo.toml not found"; fi

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

openapi-check:
  @if [ -f scripts/export-openapi.sh ]; then \
    bash scripts/export-openapi.sh; \
    if [ -f docs/openapi/openapi.json ]; then \
      status="$$(git status --short -- docs/openapi/openapi.json || true)"; \
      if [ -n "$$status" ]; then \
        echo "OpenAPI spec changed:"; \
        echo "$$status"; \
        exit 1; \
      fi; \
    else \
      echo "Skipping OpenAPI diff check: docs/openapi/openapi.json not found after export"; \
    fi \
  ; else echo "Skipping OpenAPI check: scripts/export-openapi.sh not found"; fi

db-migrate-up:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then source scripts/dev-db-env.sh && sqlx migrate run; \
    else echo "Skipping db migrate up: sqlx CLI not installed"; fi \
  ; else echo "Skipping db migrate up: migrations directory not found"; fi

db-migrate-down:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then source scripts/dev-db-env.sh && sqlx migrate revert; \
    else echo "Skipping db migrate down: sqlx CLI not installed"; fi \
  ; else echo "Skipping db migrate down: migrations directory not found"; fi

perf-smoke:
  @bash scripts/perf-smoke.sh

perf-ci:
  @PERF_ENFORCE=1 bash scripts/perf-smoke.sh

ci-local:
  @just lint
  @just sqlx-check
  @just openapi-check
  @just test
  @just perf-smoke

dev-server:
  @if [ -f Cargo.toml ]; then source scripts/dev-db-env.sh && cargo run --bin server; else echo "Skipping dev-server: Cargo workspace not initialized"; fi

dev-web:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web dev; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm dev; \
  else echo "Skipping dev-web: frontend workspace not initialized"; fi

db-reset-dev:
  @if [ -f scripts/db-reset-dev.sh ]; then bash scripts/db-reset-dev.sh; else echo "Skipping db reset: scripts/db-reset-dev.sh not found"; fi
