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

db-list-test-schemas-local:
  @if [ -f scripts/db-clean-test-schemas.sh ]; then source scripts/local-db-env.sh && bash scripts/db-clean-test-schemas.sh --dry-run; else echo "Skipping test schema listing: scripts/db-clean-test-schemas.sh not found"; fi

db-clean-test-schemas-local:
  @if [ -f scripts/db-clean-test-schemas.sh ]; then source scripts/local-db-env.sh && bash scripts/db-clean-test-schemas.sh --apply; else echo "Skipping test schema cleanup: scripts/db-clean-test-schemas.sh not found"; fi

db-list-alpha-projects-local:
  @if [ -f scripts/db-clean-alpha-projects.sh ]; then source scripts/local-db-env.sh && bash scripts/db-clean-alpha-projects.sh --dry-run; else echo "Skipping alpha project listing: scripts/db-clean-alpha-projects.sh not found"; fi

db-clean-alpha-projects-local:
  @if [ -f scripts/db-clean-alpha-projects.sh ]; then source scripts/local-db-env.sh && bash scripts/db-clean-alpha-projects.sh --apply; else echo "Skipping alpha project cleanup: scripts/db-clean-alpha-projects.sh not found"; fi

db-list-alpha-runtime-local:
  @just db-list-alpha-projects-local

db-clean-alpha-runtime-local:
  @just db-clean-alpha-projects-local

test-frontend:
  @if [ -f apps/web/package.json ]; then pnpm --dir apps/web test; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test; \
  else echo "Skipping frontend tests: package manifest not found"; fi

test-e2e:
  @if [ -f scripts/e2e-web.sh ]; then bash scripts/e2e-web.sh; \
  elif [ -f apps/web/package.json ]; then pnpm --dir apps/web test:e2e; \
  elif [ -f package.json ] || [ -f pnpm-workspace.yaml ]; then pnpm test:e2e; \
  else echo "Skipping e2e tests: package manifest not found"; fi

test-e2e-local:
  @if [ -f scripts/e2e-web.sh ]; then source scripts/local-db-env.sh && just test-e2e; \
  else echo "Skipping e2e tests: scripts/e2e-web.sh not found"; fi

alpha-smoke:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping alpha smoke: Cargo.toml not found"; \
  elif [ -z "${DATABASE_URL:-}" ]; then \
    echo "DATABASE_URL is required; use just alpha-smoke-local for local isolated test schema setup" >&2; \
    exit 1; \
  else \
    bash scripts/alpha-http.sh smoke; \
  fi

alpha-full:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping alpha full: Cargo.toml not found"; \
  elif [ -z "${DATABASE_URL:-}" ]; then \
    echo "DATABASE_URL is required; use just alpha-full-local for local isolated test schema setup" >&2; \
    exit 1; \
  else \
    bash scripts/alpha-http.sh full; \
  fi

alpha-smoke-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    export DATABASE_URL="${TEST_DATABASE_URL:-}"; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "TEST_DATABASE_URL is required; local alpha smoke runs only against isolated test schemas" >&2; \
      exit 1; \
    fi; \
    bash scripts/alpha-http.sh smoke; \
  else echo "Skipping alpha smoke: Cargo workspace not initialized"; fi

alpha-full-local:
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    export DATABASE_URL="${TEST_DATABASE_URL:-}"; \
    if [ -z "${DATABASE_URL:-}" ]; then \
      echo "TEST_DATABASE_URL is required; local alpha full runs only against isolated test schemas" >&2; \
      exit 1; \
    fi; \
    bash scripts/alpha-http.sh full; \
  else echo "Skipping alpha full: Cargo workspace not initialized"; fi

coverage:
  @if [ -f Cargo.toml ]; then cargo llvm-cov --workspace --no-cfg-coverage --ignore-filename-regex 'apps/server/src/(bin/.*|main.rs)$' --lcov --output-path target/lcov.info; else echo "Skipping backend coverage: Cargo.toml not found"; fi

coverage-check:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping backend coverage check: Cargo.toml not found"; \
  elif ! cargo llvm-cov --version >/dev/null 2>&1; then \
    echo "Skipping backend coverage check: cargo-llvm-cov is not installed"; \
  else \
    cargo llvm-cov --workspace --no-cfg-coverage --ignore-filename-regex 'apps/server/src/(bin/.*|main.rs)$' --summary-only --fail-under-lines "${COVERAGE_MIN_LINES:-36}"; \
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
    before_hash=""; \
    if [ -f docs/artifacts/openapi.json ]; then \
      before_hash="$(git hash-object docs/artifacts/openapi.json)"; \
    fi; \
    bash scripts/export-openapi.sh; \
    if [ -f docs/artifacts/openapi.json ]; then \
      after_hash="$(git hash-object docs/artifacts/openapi.json)"; \
      if [ "$before_hash" != "$after_hash" ]; then \
        echo "OpenAPI spec changed:"; \
        git status --short -- docs/artifacts/openapi.json || true; \
        git diff -- docs/artifacts/openapi.json || true; \
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

perf-web-smoke:
  @bash scripts/web-perf-smoke.sh

perf-bundle-budget:
  @bash scripts/web-bundle-budget.sh

perf-db-slow-queries:
  @bash scripts/perf-db-slow-queries.sh

perf-summary:
  @bash scripts/perf-summary.sh

perf-baseline:
  @bash scripts/perf-baseline.sh

perf-baseline-local:
  @if [ -f Cargo.toml ]; then source scripts/local-db-env.sh && just perf-baseline; else echo "Skipping perf baseline: Cargo.toml not found"; fi

release-package:
  @bash scripts/release-package.sh

release-package-check:
  @bash scripts/release-package-check.sh

staging-smoke:
  @bash scripts/staging-smoke.sh

ci-local:
  @just lint
  @just sqlx-check
  @just openapi-check
  @just test
  @just coverage-check
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
  @if [ -f Cargo.toml ]; then \
    source scripts/local-db-env.sh; \
    if [ -z "${TEST_DATABASE_URL:-}" ]; then \
      echo "TEST_DATABASE_URL is required; local full CI needs database-backed coverage" >&2; \
      exit 1; \
    fi; \
    just ci-local; \
  else \
    just ci-local; \
  fi
  @just ci-local-db
  @just test-e2e-local

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

# ---------------------------------------------------------------------------
# Coffee Demo
# ---------------------------------------------------------------------------

# Drop and recreate the demo schema, run migrations, seed fixtures, write current-run.json.
demo-coffee-reset:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping demo-coffee-reset: Cargo workspace not initialized"; \
  elif ! command -v psql >/dev/null 2>&1; then \
    echo "psql is required for demo-coffee-reset (PostgreSQL client tools)" >&2; \
    exit 1; \
  elif ! command -v sqlx >/dev/null 2>&1; then \
    echo "sqlx CLI is required for demo-coffee-reset" >&2; \
    exit 1; \
  else \
    bash scripts/demo-coffee-reset.sh; \
  fi

# Start the server pointing at the isolated coffee demo schema.
run-server-local-demo-coffee:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping run-server-local-demo-coffee: Cargo workspace not initialized"; \
  elif [ ! -f demo/coffee/generated/current-run.json ]; then \
    echo "Coffee demo not initialised — run: just demo-coffee-reset" >&2; \
    exit 1; \
  else \
    demo_url="$$(python3 -c "import json,sys; d=json.load(open('demo/coffee/generated/current-run.json')); print(d['database_url'])")"; \
    DATABASE_URL="$${demo_url}" cargo run --bin server; \
  fi

# Start config-center backend, admin UI, simulated business backends, control API, and monitor UI.
demo-coffee-up:
  @if [ ! -f Cargo.toml ]; then \
    echo "Skipping demo-coffee-up: Cargo workspace not initialized"; \
  elif [ ! -f apps/web/package.json ]; then \
    echo "Skipping demo-coffee-up: frontend workspace not initialized"; \
  else \
    bash scripts/demo-coffee-up.sh; \
  fi

# Smoke-test an already running coffee demo stack.
demo-coffee-smoke:
  @if [ ! -f demo/coffee/generated/current-run.json ]; then \
    echo "Coffee demo not initialised — start it first with: DEMO_COFFEE_RESET=1 just demo-coffee-up" >&2; \
    exit 1; \
  else \
    bash scripts/demo-coffee-smoke.sh; \
  fi

# Fast static checks for demo code, without starting the full demo stack.
demo-coffee-check:
  @bash -n scripts/demo-coffee-reset.sh scripts/demo-coffee-up.sh scripts/demo-coffee-smoke.sh
  @python3 -m py_compile scripts/demo-coffee-access-app.py
  @cargo fmt --all --check
  @cargo check --bin demo-coffee-seed
  @pnpm --dir demo/coffee/monitor exec tsc --noEmit
  @pnpm --dir demo/coffee/monitor build

# Tear down the demo schema (idempotent, safe to call anytime).
demo-coffee-down:
  @if ! command -v psql >/dev/null 2>&1; then \
    echo "psql is required for demo-coffee-down" >&2; \
    exit 1; \
  else \
    source scripts/local-db-env.sh; \
    base_url="$${DATABASE_URL%%\?*}"; \
    psql "$${base_url}" -c "DROP SCHEMA IF EXISTS mini_conf_demo_coffee CASCADE;" -q && \
    echo "Schema mini_conf_demo_coffee dropped."; \
  fi
