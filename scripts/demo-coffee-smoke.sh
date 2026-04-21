#!/usr/bin/env bash
# Smoke-test an already running coffee demo stack.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_file="${repo_root}/demo/coffee/generated/current-run.json"

cd "${repo_root}"

if [[ ! -f "${run_file}" ]]; then
  echo "Coffee demo is not initialised. Start it first with: DEMO_COFFEE_RESET=1 just demo-coffee-up" >&2
  exit 1
fi

python3 - <<'PY'
from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

BASE = "http://127.0.0.1"
CONTROL = f"{BASE}:19010"


def fail(message: str) -> None:
    print(f"[FAIL] {message}", file=sys.stderr)
    raise SystemExit(1)


def request(method: str, url: str, payload: dict[str, Any] | None = None, timeout: float = 3.0) -> tuple[int, Any]:
    data = None
    headers: dict[str, str] = {"Accept": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            body = response.read().decode("utf-8")
            try:
                parsed: Any = json.loads(body) if body else {}
            except json.JSONDecodeError:
                parsed = body
            return response.status, parsed
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8")
        try:
            parsed: Any = json.loads(body) if body else {}
        except json.JSONDecodeError:
            parsed = body
        return exc.code, parsed
    except Exception as exc:  # noqa: BLE001
        fail(f"{method} {url} failed: {exc}")


def assert_status(label: str, method: str, url: str, expected: int = 200, payload: dict[str, Any] | None = None) -> Any:
    status, body = request(method, url, payload=payload)
    if status != expected:
        fail(f"{label}: expected HTTP {expected}, got {status}: {body}")
    print(f"[OK] {label}")
    return body


def assert_frontend(label: str, url: str) -> None:
    try:
        with urllib.request.urlopen(url, timeout=3) as response:
            if response.status >= 500:
                fail(f"{label}: expected a non-5xx response, got {response.status}")
            print(f"[OK] {label}")
    except Exception as exc:  # noqa: BLE001
        fail(f"{label}: {exc}")


assert_status("config center health", "GET", f"{BASE}:8080/api/healthz")
assert_frontend("admin web frontend", f"{BASE}:5173")
assert_frontend("demo monitor frontend", f"{BASE}:5174")

bootstrap_a = assert_status(
    "backend-a SN001 bootstrap",
    "GET",
    f"{BASE}:19001/api/bootstrap/config-center?{urllib.parse.urlencode({'sn': 'SN001'})}",
)
bootstrap_b = assert_status(
    "backend-b SN001 bootstrap",
    "GET",
    f"{BASE}:19002/api/bootstrap/config-center?{urllib.parse.urlencode({'sn': 'SN001'})}",
)
if bootstrap_a.get("deployment_key") != "a-prod-store-001":
    fail(f"backend-a SN001 mapped to {bootstrap_a.get('deployment_key')!r}")
if bootstrap_b.get("deployment_key") != "b-prod-store-001":
    fail(f"backend-b SN001 mapped to {bootstrap_b.get('deployment_key')!r}")

state = assert_status("demo control API", "GET", f"{CONTROL}/api/demo/state")
if state.get("project_code") != "coffee-middleware-demo":
    fail(f"unexpected project_code: {state.get('project_code')!r}")
clients = state.get("clients") or {}
if "a-prod-store-001" not in clients:
    fail("expected seeded virtual client a-prod-store-001")
bindings = state.get("sn_bindings") or {}
if bindings.get("backend-a", {}).get("SN001") != "a-prod-store-001":
    fail("backend-a SN001 binding missing")
if bindings.get("backend-b", {}).get("SN001") != "b-prod-store-001":
    fail("backend-b SN001 binding missing")
print("[OK] seeded state")

assert_status(
    "bind smoke SN",
    "POST",
    f"{CONTROL}/api/demo/backends/backend-a/sn-bindings",
    payload={"sn": "SN_SMOKE", "deployment_key": "a-prod-store-001"},
)
state = assert_status("read state after bind", "GET", f"{CONTROL}/api/demo/state")
if state.get("sn_bindings", {}).get("backend-a", {}).get("SN_SMOKE") != "a-prod-store-001":
    fail("SN_SMOKE was not bound to a-prod-store-001")
assert_status("unbind smoke SN", "DELETE", f"{CONTROL}/api/demo/backends/backend-a/sn-bindings/SN_SMOKE")
state = assert_status("read state after unbind", "GET", f"{CONTROL}/api/demo/state")
if "SN_SMOKE" in state.get("sn_bindings", {}).get("backend-a", {}):
    fail("SN_SMOKE was not removed")
print("[OK] SN bind/unbind control API")

deadline = time.monotonic() + 12
while time.monotonic() < deadline:
    state = assert_status("demo control cache", "GET", f"{CONTROL}/api/demo/state")
    if state.get("cc_instances") and state.get("cc_releases"):
        break
    time.sleep(1)
else:
    fail("control API did not cache config-center instances and releases")
print("[OK] config-center cache")

assert_status("clear demo events", "POST", f"{CONTROL}/api/demo/events/clear", payload={})
for action in ("bootstrap", "pull", "apply", "heartbeat"):
    body = assert_status(
        f"manual client {action}",
        "POST",
        f"{CONTROL}/api/demo/clients/a-prod-store-001/{action}",
        payload={},
    )
    if body.get("ok") is not True:
        fail(f"manual client {action} returned non-ok body: {body}")

deadline = time.monotonic() + 10
latest: dict[str, Any] = {}
while time.monotonic() < deadline:
    latest = assert_status("read state after manual flow", "GET", f"{CONTROL}/api/demo/state")
    client = (latest.get("clients") or {}).get("a-prod-store-001") or {}
    configs = latest.get("effective_configs") or {}
    events = latest.get("events") or []
    if client.get("last_revisions", {}).get("coffee-main") and configs and events:
        break
    time.sleep(0.5)
else:
    fail("manual flow did not produce revisions, effective configs, and events")

client = latest["clients"]["a-prod-store-001"]
if "store-flags" not in client.get("last_revisions", {}):
    fail("manual flow did not apply store-flags")
event_kinds = {event.get("kind") for event in latest.get("events", [])}
for expected_kind in ("bootstrap", "bundle_pulled", "client_applied", "heartbeat"):
    if expected_kind not in event_kinds:
        fail(f"missing event kind {expected_kind!r}; got {sorted(event_kinds)}")
print("[OK] manual bootstrap -> pull -> apply -> heartbeat flow")

print()
print("Coffee demo smoke passed.")
PY
