# Coffee Demo

End-to-end demo of the config-center onboarding flow.
It models a production-style chain with human-operated admin steps and manually triggered client actions:

```text
Admin UI publishes config
   |
   v
mini-conf config center (127.0.0.1:8080)
   |
   | business backend bootstraps old clients by SN
   v
Backend-A (127.0.0.1:19001) / Backend-B (127.0.0.1:19002)
   |
   | Demo Monitor buttons trigger bootstrap -> pull -> apply -> heartbeat
   v
Virtual clients write effective configs and report telemetry
```

The demo is intentionally not a fully automatic playback. The monitor gives you buttons for each runtime action so a live presentation can pause, explain, and show the data moving through the system.

## Data Model

| Layer        | Value                                        |
| ------------ | -------------------------------------------- |
| Project      | `coffee-middleware-demo`                     |
| Environments | `dev`, `prod`                                |
| Config files | `coffee-main`, `store-flags`, `store-secret` |
| Template     | `tpl-coffee-store-basic`                     |

### Instances

| Instance key       | Backend   | SN    | Status   | Purpose                    |
| ------------------ | --------- | ----- | -------- | -------------------------- |
| `a-prod-store-001` | backend-a | SN001 | active   | Happy-path lifecycle demo  |
| `a-prod-store-002` | backend-a | SN002 | inactive | Lifecycle gating demo      |
| `b-prod-store-001` | backend-b | SN001 | active   | Same SN, different backend |

`SN001` appears on both backend-a and backend-b. That is intentional: SN resolution is scoped by backend, so the same physical-looking SN can route to different deployment instances in different integration surfaces.

### Credentials

Plaintext tokens are demo-only fixtures:

| Instance           | Token                       |
| ------------------ | --------------------------- |
| `a-prod-store-001` | `mc_demo_coffee_a_prod_001` |
| `a-prod-store-002` | `mc_demo_coffee_a_prod_002` |
| `b-prod-store-001` | `mc_demo_coffee_b_prod_001` |

Admin login:

```toml
username = "admin"
password = "admin123456"
```

## Quick Start

```bash
just demo-coffee-up
```

This starts the full local stack:

| Service               | URL                                                           |
| --------------------- | ------------------------------------------------------------- |
| Config center backend | `http://127.0.0.1:8080`                                       |
| Admin UI              | `http://127.0.0.1:5173`                                       |
| Demo Monitor          | `http://127.0.0.1:5174`                                       |
| backend-a bootstrap   | `http://127.0.0.1:19001/api/bootstrap/config-center?sn=SN001` |
| backend-b bootstrap   | `http://127.0.0.1:19002/api/bootstrap/config-center?sn=SN001` |
| Demo control API      | `http://127.0.0.1:19010/api/demo/state`                       |

If `demo/coffee/generated/current-run.json` is missing, startup initializes the isolated schema first. To force a clean reset:

```bash
DEMO_COFFEE_RESET=1 just demo-coffee-up
```

Runtime logs are written to `demo/coffee/generated/logs/`.

## Smoke Check

Run this in another terminal while `just demo-coffee-up` is still running:

```bash
just demo-coffee-smoke
```

The smoke check verifies that all four surfaces are reachable, SN routing is correct, and one seeded client can complete:

```text
bootstrap -> pull bundle -> apply effective config -> report heartbeat
```

It also tests the monitor control API for SN bind/unbind. If you have changed activation tokens from the Admin UI, run a clean reset first or paste the new token into the matching client card in the monitor.

It is a readiness check, not the full demo script.

## CI Level

The coffee demo is intentionally kept out of the default full CI runtime flow for now. A complete smoke run starts the config center, Admin UI, demo control API, two bootstrap backends, and the monitor UI, then mutates an isolated PostgreSQL schema. That is useful for local readiness, but too heavy and timing-sensitive for the main CI gate while the demo backend is still settling.

Use this fast check before committing demo changes:

```bash
just demo-coffee-check
```

It validates shell syntax, Python syntax, the Rust seed binary, and the standalone monitor build without starting long-running services. Use `DEMO_COFFEE_RESET=1 just demo-coffee-up` plus `just demo-coffee-smoke` for manual end-to-end verification.

## Manual Startup

Use separate terminals when you want finer control:

```bash
# 1. Reset demo schema, run migrations, seed fixtures
just demo-coffee-reset

# 2. Start server against the demo schema
just run-server-local-demo-coffee

# 3. Start the admin UI
just dev-web

# 4. Start backend-a, backend-b, and the control API
python3 scripts/demo-coffee-access-app.py serve

# 5. Start the standalone monitor UI
pnpm --dir demo/coffee/monitor dev --host 127.0.0.1
```

## One-Line Reset

```bash
just demo-coffee-reset
```

This does: drop schema -> create schema -> migrations -> seed -> write `generated/current-run.json`.

## Tear Down

```bash
just demo-coffee-down
```

Drops PostgreSQL schema `mini_conf_demo_coffee`. It does not touch the daily dev schema or test schemas.

## Demo Script

### 1. Open The Two Screens

Start the stack and open:

- Admin UI: `http://127.0.0.1:5173`
- Demo Monitor: `http://127.0.0.1:5174`

In the Admin UI, verify project `coffee-middleware-demo`, config file `coffee-main`, template `tpl-coffee-store-basic`, and instance `a-prod-store-001`.

### 2. Bootstrap And Apply One Existing Store

In the Demo Monitor, use `a-prod-store-001`:

1. Click Bootstrap.
2. Click Pull Bundle.
3. Click Apply.
4. Click Heartbeat.

The monitor should show effective config content, client revisions, and timeline events. The Admin UI should show sync and heartbeat records after the control API cache refreshes.

### 3. Publish A Config Change

In the Admin UI:

1. Open `a-prod-store-001`.
2. Edit `coffee-main` Draft.
3. Preview bundle.
4. Publish a new Release.

Return to the Demo Monitor. The latest release for `coffee-main` should move ahead of the client-applied revision, making the store look stale until the client pulls again.

### 4. Catch The Client Up

In the Demo Monitor for `a-prod-store-001`:

1. Click Pull Bundle.
2. Click Apply.
3. Click Heartbeat.

The stale marker should clear when the applied revision catches up.

### 5. Clone A New Store

In the Admin UI:

1. Clone template `tpl-coffee-store-basic` into a new deployment instance such as `a-prod-store-003`.
2. Activate it and copy the one-time token.

In the Demo Monitor:

1. Bind `backend-a / SN003` to `a-prod-store-003`.
2. Create or select virtual client `a-prod-store-003`. Use the deployment key as the client id.
3. Paste the activation token into the client card's `Activation token` field.
4. Click Bootstrap, Pull Bundle, Apply, and Heartbeat.

This demonstrates the "copy one store configuration and onboard a new store" flow.

### Token Rule During Manual Testing

Seeded clients start with the fixture tokens from `current-run.json`. If you activate an instance again or reset its token from the Admin UI, the old fixture token becomes invalid. The monitor cannot infer that secret from the config center, so paste the new one into the matching client card before Pull Bundle, Apply, or Heartbeat.

### 6. Lifecycle And Failure States

Use `a-prod-store-002` or another inactive instance:

1. In the Admin UI, archive an inactive instance and confirm it leaves the default list.
2. Restore it from Archived Instances.
3. Permanently delete only when the strict delete rules allow it.
4. Try a monitor action against an inactive or unavailable instance to show a clear failure event.

## Generated Files

`demo/coffee/generated/` is git-ignored and recreated by reset/startup:

| File or directory       | Contents                                                             |
| ----------------------- | -------------------------------------------------------------------- |
| `current-run.json`      | Demo database URL, admin credentials, token map, SN routing table    |
| `runtime-bindings.json` | Mutable SN-to-deployment bindings changed from the monitor           |
| `effective-configs/`    | Config files written by virtual clients after Apply                  |
| `logs/`                 | Server, admin UI, access app, and monitor logs from `demo-coffee-up` |

## File Layout

```text
demo/coffee/
  README.md
  manifest.yaml
  fixtures/
    README.md
    templates/
      coffee-main.toml
  clients/
    store-a-prod-001.local.toml
    store-a-prod-002.local.toml
  monitor/
    src/
      CoffeeDemoMonitor.vue
      api.ts
      style.css
  generated/
    current-run.json
    runtime-bindings.json
    effective-configs/
    logs/
```

## Schema Isolation

The demo lives in PostgreSQL schema `mini_conf_demo_coffee`.
It is safe to reset repeatedly and does not touch ordinary local development data.
