# AGENTS

## Rust Test Conventions

- For Rust files under `apps/server/tests/**` and `crates/**/tests/**`, avoid `unwrap()` and `expect()` in setup, teardown, seed helpers, and JSON/body parsing helpers.
- Prefer `type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>` and propagate failures with `?`.
- Test functions should return `TestResult` when practical.
- Prefer `assert!` and `assert_eq!` for final behavior checks instead of panic-style extraction.
- If a panic-style API is unavoidable, keep it at the narrowest leaf and avoid introducing it into shared helpers.

## Open API Test Pattern

- Database-backed integration tests should use an isolated schema per test.
- Shared setup helpers should return `Result<Option<...>>` so tests can cleanly early-return when `TEST_DATABASE_URL` is absent.
- Teardown helpers should return `Result` and always drop the temporary schema.
