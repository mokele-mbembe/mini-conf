/* global console, process */

const managedServer = process.env.E2E_MANAGED_SERVER === "1";
const allowSharedServer = process.env.E2E_ALLOW_SHARED_SERVER === "1";

if (!managedServer && !allowSharedServer) {
  console.error(
    [
      "Refusing to run web e2e against an implicit shared server.",
      "",
      "Use `just test-e2e-local` for an isolated temporary backend/frontend/database schema.",
      "If you intentionally want to target an already running server, set:",
      "  E2E_ALLOW_SHARED_SERVER=1 PLAYWRIGHT_BASE_URL=http://127.0.0.1:5173 pnpm --dir apps/web test:e2e",
    ].join("\n"),
  );
  process.exit(1);
}
