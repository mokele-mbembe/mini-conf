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
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping backend db tests: Cargo.toml not found"; \
  elif [ -z "${TEST_DATABASE_URL:-}" ]; then \
    echo "TEST_DATABASE_URL is required; use just test-backend-db-local for developer-local resolution" >&2; \
    exit 1; \
  else \
    cargo nextest run --workspace; \
  fi

test-backend-db-local:
  @if [ -f Cargo.toml ]; then source scripts/local-db-env.sh && just test-backend-db; else echo "Skipping backend db tests: Cargo.toml not found"; fi

test-frontend:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web test; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test; \
  else echo "Skipping frontend tests: package manifest not found"; fi

test-e2e:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web test:e2e; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test:e2e; \
  else echo "Skipping e2e tests: package manifest not found"; fi

alpha-smoke:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping alpha smoke: Cargo.toml not found"; \
  elif [ -z "${DATABASE_URL:-}" ]; then \
    echo "DATABASE_URL is required; use just alpha-smoke-local for current developer-local test DB reuse" >&2; \
    exit 1; \
  else \
    bash scripts/alpha-http.sh smoke; \
  fi

alpha-full:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping alpha full: Cargo.toml not found"; \
  elif [ -z "${DATABASE_URL:-}" ]; then \
    echo "DATABASE_URL is required; use just alpha-full-local for current developer-local test DB reuse" >&2; \
    exit 1; \
  else \
    bash scripts/alpha-http.sh full; \
  fi

alpha-smoke-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    export DATABASE_URL="${DATABASE_URL:-${TEST_DATABASE_URL:-}}"; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "DATABASE_URL is required; local test DB resolution did not produce a usable DSN" >&2; \
      exit 1; \
    fi; \
    bash scripts/alpha-http.sh smoke; \
  else echo "Skipping alpha smoke: Cargo workspace not initialized"; fi

alpha-full-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    export DATABASE_URL="${DATABASE_URL:-${TEST_DATABASE_URL:-}}"; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "DATABASE_URL is required; local test DB resolution did not produce a usable DSN" >&2; \
      exit 1; \
    fi; \
    bash scripts/alpha-http.sh full; \
  else echo "Skipping alpha full: Cargo workspace not initialized"; fi

coverage:
  @if [ -f Cargo.toml ]; then cargo llvm-cov --workspace --lcov --output-path target/lcov.info; else echo "Skipping backend coverage: Cargo.toml not found"; fi

coverage-check:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping backend coverage check: Cargo.toml not found"; \
  elif ! cargo llvm-cov --version >/dev/null 2>&1; then \
    echo "Skipping backend coverage check: cargo-llvm-cov is not installed"; \
  else \
    cargo llvm-cov --workspace --summary-only --fail-under-lines "${COVERAGE_MIN_LINES:-39}"; \
  fi

sqlx-check:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping sqlx prepare check: Cargo.toml not found"; \
  elif [ -d .sqlx ] || rg -n 'query!|query_as!|query_scalar!|query_file!|query_file_as!|query_file_scalar!' apps/server crates >/dev/null 2>&1; then \
    cargo sqlx prepare --check --workspace; \
  else \
    echo "Skipping sqlx prepare check: no compile-time SQLx query metadata in this workspace"; \
  fi

openapi-check:
  @if [ -f scripts/export-openapi.sh ]; then \
    bash scripts/export-openapi.sh; \
    if [ -f docs/artifacts/openapi.json ]; then \
      if ! git diff --quiet -- docs/artifacts/openapi.json; then \
        echo "OpenAPI spec changed:"; \
        git status --short -- docs/artifacts/openapi.json || true; \
        exit 1; \
      fi; \
    else \
      echo "Skipping OpenAPI diff check: docs/artifacts/openapi.json not found after export"; \
    fi \
  ; else echo "Skipping OpenAPI check: scripts/export-openapi.sh not found"; fi

db-migrate-up:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then \
      if [ -z "${DATABASE_URL:-}" ]; then \
        echo "DATABASE_URL is required; use just db-migrate-up-local for developer-local resolution" >&2; \
        exit 1; \
      fi; \
      sqlx migrate run; \
    else echo "Skipping db migrate up: sqlx CLI not installed"; fi \
  ; else echo "Skipping db migrate up: migrations directory not found"; fi

db-migrate-up-local:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then source scripts/local-db-env.sh && just db-migrate-up; \
    else echo "Skipping db migrate up: sqlx CLI not installed"; fi \
  ; else echo "Skipping db migrate up: migrations directory not found"; fi

dev-seed-demo:
  @if [ -f Cargo.toml ]; then bash scripts/dev-seed-demo.sh; else echo "Skipping demo seed: Cargo workspace not initialized"; fi

dev-seed-demo-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "DATABASE_URL is required for local demo seed; configure MINI_CONF_LOCAL_DB_* or MINI_CONF_LOCAL_DATABASE_URL" >&2; \
      exit 1; \
    fi; \
    if [ -n "${TEST_DATABASE_URL:-}" ] && [ "${DATABASE_URL}" = "${TEST_DATABASE_URL}" ]; then \
      echo "Warning: DATABASE_URL and TEST_DATABASE_URL currently point to the same database; separate runtime and test DBs are recommended for Local Preview / UI Dev" >&2; \
    fi; \
    bash scripts/dev-seed-demo.sh; \
  else echo "Skipping demo seed: Cargo workspace not initialized"; fi

dev-db-prepare-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "DATABASE_URL is required for local preview DB; configure MINI_CONF_LOCAL_DB_* or MINI_CONF_LOCAL_DATABASE_URL" >&2; \
      exit 1; \
    fi; \
    if [ -n "${TEST_DATABASE_URL:-}" ] && [ "${DATABASE_URL}" = "${TEST_DATABASE_URL}" ]; then \
      echo "Warning: DATABASE_URL and TEST_DATABASE_URL currently point to the same database; separate runtime and test DBs are recommended for Local Preview / UI Dev" >&2; \
    fi; \
    just db-migrate-up; \
    bash scripts/dev-seed-demo.sh; \
  else echo "Skipping local preview DB prepare: Cargo workspace not initialized"; fi

db-migrate-down:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then \
      if [ -z "${DATABASE_URL:-}" ]; then \
        echo "DATABASE_URL is required; use just db-migrate-down-local for developer-local resolution" >&2; \
        exit 1; \
      fi; \
      sqlx migrate revert; \
    else echo "Skipping db migrate down: sqlx CLI not installed"; fi \
  ; else echo "Skipping db migrate down: migrations directory not found"; fi

db-migrate-down-local:
  @if [ -d migrations ]; then \
    if command -v sqlx >/dev/null 2>&1; then source scripts/local-db-env.sh && just db-migrate-down; \
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
  @just coverage-check
  @just test
  @just perf-smoke

ci-local-db:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    export DATABASE_URL="${DATABASE_URL:-${TEST_DATABASE_URL:-}}"; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "DATABASE_URL is required; local DB resolution did not produce a usable runtime or test DSN" >&2; \
      exit 1; \
    fi; \
    if [ -z "${TEST_DATABASE_URL:-}" ]; then \
      echo "TEST_DATABASE_URL is required; local DB resolution did not produce a usable test DSN" >&2; \
      exit 1; \
    fi; \
    just db-migrate-up; \
    just test-backend-db; \
  else echo "Skipping local DB CI: Cargo workspace not initialized"; fi

ci-local-full:
  @just ci-local
  @just ci-local-db

run-server:
  @if [ -f Cargo.toml ]; then cargo run --bin server; else echo "Skipping run-server: Cargo workspace not initialized"; fi

run-server-local:
  @if [ -f Cargo.toml ]; then source scripts/local-db-env.sh && just run-server; else echo "Skipping run-server: Cargo workspace not initialized"; fi

dev-server:
  @just run-server-local

dev-web:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web dev; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm dev; \
  else echo "Skipping dev-web: frontend workspace not initialized"; fi

db-reset-dev:
  @if [ -f scripts/db-reset-dev.sh ]; then bash scripts/db-reset-dev.sh; else echo "Skipping db reset: scripts/db-reset-dev.sh not found"; fi
