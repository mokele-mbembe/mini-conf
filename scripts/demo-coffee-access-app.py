#!/usr/bin/env python3
"""Coffee demo backend + control API server.

Runs three HTTP servers in one process (all stdlib, no extra dependencies):

  backend-a  :19001  — bootstrap endpoint for backend-a
  backend-b  :19002  — bootstrap endpoint for backend-b
  control    :19010  — demo control API consumed by the monitor UI

The backend servers implement:
  GET /api/bootstrap/config-center?sn=SN001
  → returns {config_center_base_url, project, environment, deployment_key, token, configs}

The control API implements:
  GET  /api/demo/state
  POST /api/demo/clients                          create a new virtual client
  POST /api/demo/clients/:id/bootstrap            trigger one bootstrap
  POST /api/demo/clients/:id/pull                 pull config-bundle
  POST /api/demo/clients/:id/apply                apply bundle + report sync record
  POST /api/demo/clients/:id/heartbeat            send heartbeat
  POST /api/demo/clients/:id/token                set activation token
  POST /api/demo/clients/:id/poll/start           start background poll loop
  POST /api/demo/clients/:id/poll/pause           pause poll loop
  POST /api/demo/backends/:backend/sn-bindings    bind SN → deployment_key
  DELETE /api/demo/backends/:backend/sn-bindings/:sn
  POST /api/demo/events/clear
"""

from __future__ import annotations

import argparse
import http.cookiejar
import json
import signal
import sys
import threading
import time
import traceback
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

# ── Paths ────────────────────────────────────────────────────────────────────
REPO_ROOT = Path(__file__).resolve().parents[1]
RUN_FILE = REPO_ROOT / "demo/coffee/generated/current-run.json"
GENERATED_DIR = REPO_ROOT / "demo/coffee/generated"
EFFECTIVE_DIR = GENERATED_DIR / "effective-configs"
BINDINGS_FILE = GENERATED_DIR / "runtime-bindings.json"

BACKENDS = {
    "backend-a": ("127.0.0.1", 19001),
    "backend-b": ("127.0.0.1", 19002),
}
CONTROL_ADDR = ("127.0.0.1", 19010)
MAX_EVENTS = 200

# ── Helpers ──────────────────────────────────────────────────────────────────

def now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def load_run() -> dict[str, Any]:
    if not RUN_FILE.exists():
        raise SystemExit("Coffee demo not initialised. Run: just demo-coffee-reset")
    with RUN_FILE.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def request_json(
    method: str,
    url: str,
    payload: dict[str, Any] | None = None,
    token: str | None = None,
    timeout: float = 8.0,
) -> tuple[int, dict[str, Any]]:
    data = None
    headers: dict[str, str] = {"Accept": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, (json.loads(body) if body else {})
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8")
        try:
            parsed = json.loads(body) if body else {}
        except json.JSONDecodeError:
            parsed = {"message": body}
        return exc.code, parsed


def request_json_with_opener(
    opener: urllib.request.OpenerDirector,
    method: str,
    url: str,
    payload: dict[str, Any] | None = None,
    timeout: float = 8.0,
) -> tuple[int, dict[str, Any]]:
    data = None
    headers: dict[str, str] = {"Accept": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with opener.open(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, (json.loads(body) if body else {})
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8")
        try:
            parsed = json.loads(body) if body else {}
        except json.JSONDecodeError:
            parsed = {"message": body}
        return exc.code, parsed


# ── Shared mutable state (lock-protected) ────────────────────────────────────

class DemoState:
    def __init__(self, run: dict[str, Any]) -> None:
        self._lock = threading.Lock()
        self.run = run
        base_url: str = run["config_center_url"].rstrip("/")
        self.config_center_url = base_url

        # SN bindings: loaded from bindings file, falls back to run sn_routing
        self.sn_bindings: dict[str, dict[str, str]] = self._load_bindings(run)

        # Virtual clients
        self.clients: dict[str, ClientState] = {}

        # Event log
        self.events: list[dict[str, Any]] = []

        # Cached config-center data (polled in background)
        self.cc_instances: list[dict[str, Any]] = []
        self.cc_releases: list[dict[str, Any]] = []
        self.cc_heartbeats: list[dict[str, Any]] = []
        self.cc_sync_records: list[dict[str, Any]] = []

        # Admin session cookie (obtained on startup, refreshed on 401)
        self.admin_cookie_jar = http.cookiejar.CookieJar()
        self.admin_opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self.admin_cookie_jar)
        )
        self.admin_session_ready = False
        self.project_id: int | None = run.get("project_id")

        # Pre-populate clients from run file
        for deployment_key, token in run.get("tokens", {}).items():
            dep_key_dashed = deployment_key.replace("_", "-")
            backend_name, sn = self._find_routing(dep_key_dashed)
            self.clients[dep_key_dashed] = ClientState(
                client_id=dep_key_dashed,
                backend_name=backend_name,
                sn=sn,
                token=token,
            )

    @staticmethod
    def _load_bindings(run: dict[str, Any]) -> dict[str, dict[str, str]]:
        if BINDINGS_FILE.exists():
            try:
                return json.loads(BINDINGS_FILE.read_text(encoding="utf-8"))
            except Exception:
                pass
        return {k: dict(v) for k, v in run.get("sn_routing", {}).items()}

    def _save_bindings(self) -> None:
        GENERATED_DIR.mkdir(parents=True, exist_ok=True)
        BINDINGS_FILE.write_text(
            json.dumps(self.sn_bindings, ensure_ascii=False, indent=2), encoding="utf-8"
        )

    def _find_routing(self, deployment_key: str) -> tuple[str, str]:
        for backend, bindings in self.sn_bindings.items():
            for sn, dep_key in bindings.items():
                if dep_key == deployment_key:
                    return backend, sn
        return "backend-a", "—"

    def append_event(self, **kwargs: Any) -> None:
        with self._lock:
            event = {"at": now_iso(), **kwargs}
            self.events.append(event)
            if len(self.events) > MAX_EVENTS:
                self.events = self.events[-MAX_EVENTS:]

    def clear_events(self) -> None:
        with self._lock:
            self.events.clear()

    def snapshot(self) -> dict[str, Any]:
        with self._lock:
            return {
                "config_center_url": self.config_center_url,
                "project_id": self.project_id,
                "project_code": self.run.get("project_code", ""),
                "sn_bindings": {k: dict(v) for k, v in self.sn_bindings.items()},
                "clients": {cid: c.to_dict() for cid, c in self.clients.items()},
                "events": list(self.events),
                "effective_configs": self._read_effective_configs(),
                "cc_instances": list(self.cc_instances),
                "cc_releases": list(self.cc_releases),
                "cc_heartbeats": list(self.cc_heartbeats),
                "cc_sync_records": list(self.cc_sync_records),
            }

    @staticmethod
    def _read_effective_configs() -> dict[str, str]:
        result: dict[str, str] = {}
        if EFFECTIVE_DIR.exists():
            paths = (
                sorted(EFFECTIVE_DIR.glob("*.yaml"))
                + sorted(EFFECTIVE_DIR.glob("*.json"))
                + sorted(EFFECTIVE_DIR.glob("*.toml"))
            )
            for f in paths:
                try:
                    content = f.read_text(encoding="utf-8")
                    if len(content) < 8192:
                        result[f.name] = content
                except Exception:
                    pass
        return result

    def bind_sn(self, backend: str, sn: str, deployment_key: str) -> None:
        with self._lock:
            if backend not in self.sn_bindings:
                self.sn_bindings[backend] = {}
            self.sn_bindings[backend][sn] = deployment_key
            self._save_bindings()

    def unbind_sn(self, backend: str, sn: str) -> None:
        with self._lock:
            self.sn_bindings.get(backend, {}).pop(sn, None)
            self._save_bindings()

    def get_deployment_key(self, backend: str, sn: str) -> str | None:
        with self._lock:
            return self.sn_bindings.get(backend, {}).get(sn)

    def get_token(self, deployment_key: str) -> str | None:
        with self._lock:
            client = self.clients.get(deployment_key)
            if client and client.token:
                return client.token
            key_under = deployment_key.replace("-", "_")
            return self.run.get("tokens", {}).get(key_under)

    def ensure_client(self, client_id: str, backend_name: str, sn: str) -> "ClientState":
        with self._lock:
            if client_id not in self.clients:
                self.clients[client_id] = ClientState(
                    client_id=client_id, backend_name=backend_name, sn=sn
                )
            return self.clients[client_id]

    def get_client(self, client_id: str) -> "ClientState | None":
        with self._lock:
            return self.clients.get(client_id)


class ClientState:
    def __init__(self, client_id: str, backend_name: str, sn: str, token: str | None = None) -> None:
        self.client_id = client_id
        self.backend_name = backend_name
        self.sn = sn
        self.token = token
        self.status: str = "idle"
        self.last_bootstrap: dict[str, Any] | None = None
        self.last_bundle: dict[str, Any] | None = None
        self.last_revisions: dict[str, str] = {}
        self.last_error: str | None = None
        self._poll_thread: threading.Thread | None = None
        self._stop_poll = threading.Event()

    def to_dict(self) -> dict[str, Any]:
        return {
            "client_id": self.client_id,
            "backend_name": self.backend_name,
            "sn": self.sn,
            "status": self.status,
            "token": "***" if self.token else None,
            "last_bootstrap": self.last_bootstrap,
            "last_bundle": None,
            "last_revisions": dict(self.last_revisions),
            "last_error": self.last_error,
        }

    def start_poll(self, state: "DemoState", interval: int = 5) -> None:
        if self.status == "polling":
            return
        self._stop_poll.clear()
        self.status = "polling"

        def _loop() -> None:
            while not self._stop_poll.wait(interval):
                if self._stop_poll.is_set():
                    break
                try:
                    _client_do_bootstrap(self, state, silent=True)
                    _client_do_pull(self, state, silent=True)
                    _client_do_apply(self, state, silent=True)
                    _client_do_heartbeat(self, state, silent=True)
                except Exception as exc:  # noqa: BLE001
                    self.last_error = str(exc)

        self._poll_thread = threading.Thread(target=_loop, daemon=True)
        self._poll_thread.start()

    def pause_poll(self) -> None:
        self._stop_poll.set()
        self.status = "paused"


# ── Client action helpers ─────────────────────────────────────────────────────

def _backend_bootstrap_url(state: DemoState, client: ClientState) -> str:
    ports = {"backend-a": 19001, "backend-b": 19002}
    port = ports.get(client.backend_name, 19001)
    qs = urllib.parse.urlencode({"sn": client.sn})
    return f"http://127.0.0.1:{port}/api/bootstrap/config-center?{qs}"


def _client_do_bootstrap(client: ClientState, state: DemoState, silent: bool = False) -> str:
    url = _backend_bootstrap_url(state, client)
    status, body = request_json("GET", url)
    if status == 200:
        client.last_bootstrap = body
        client.last_error = None
        if client.status not in ("polling",):
            client.status = "bootstrapped"
        if not silent:
            state.append_event(
                kind="bootstrap",
                client=client.client_id,
                backend=client.backend_name,
                sn=client.sn,
                deployment_key=body.get("deployment_key"),
            )
        return "ok"
    else:
        msg = body.get("message", f"HTTP {status}")
        client.last_error = msg
        client.status = "error"
        if not silent:
            state.append_event(
                kind="bootstrap_failed",
                client=client.client_id,
                sn=client.sn,
                message=msg,
            )
        return msg


def _client_do_pull(client: ClientState, state: DemoState, silent: bool = False) -> str:
    bs = client.last_bootstrap
    if not bs:
        return "not bootstrapped"
    base = bs["config_center_base_url"].rstrip("/")
    dep_key = bs["deployment_key"]
    token = client.token or bs.get("token") or state.get_token(dep_key)
    if not token:
        return "no token"
    qs = urllib.parse.urlencode({"project": bs["project"], "environment": bs["environment"]})
    url = f"{base}/api/open/deployments/{dep_key}/config-bundle?{qs}"
    status, body = request_json("GET", url, token=token)
    if status == 200:
        client.last_bundle = body
        client.last_error = None
        if not silent:
            state.append_event(kind="bundle_pulled", client=client.client_id, deployment_key=dep_key)
        return "ok"
    else:
        msg = body.get("message", f"HTTP {status}")
        client.last_error = msg
        client.status = "error"
        if not silent:
            state.append_event(kind="bundle_failed", client=client.client_id, deployment_key=dep_key, message=msg, status="failed")
        return msg


def _client_do_apply(client: ClientState, state: DemoState, silent: bool = False) -> str:
    bundle = client.last_bundle
    bs = client.last_bootstrap
    if not bundle or not bs:
        return "no bundle"
    dep_key = bs["deployment_key"]
    token = client.token or bs.get("token") or state.get_token(dep_key)
    base = bs["config_center_base_url"].rstrip("/")

    EFFECTIVE_DIR.mkdir(parents=True, exist_ok=True)
    applied: list[str] = []

    for cfg in bundle.get("configs", []):
        config_code: str = cfg["config"]
        revision: str = cfg["revision"]
        content: str = cfg["content"]
        fmt: str = cfg.get("format", "yaml")
        out_path = EFFECTIVE_DIR / f"{client.client_id}.{config_code}.{fmt}"
        out_path.write_text(content, encoding="utf-8")

        changed = client.last_revisions.get(config_code) != revision
        client.last_revisions[config_code] = revision
        action = "apply" if changed else "version_check"

        sync_payload = {
            "project": bs["project"],
            "environment": bs["environment"],
            "deployment_key": dep_key,
            "config": config_code,
            "action": action,
            "revision": revision,
            "status": "success",
            "message": "applied" if changed else "already current",
            "detail": {"client": client.client_id, "source": "demo-monitor"},
            "reported_at": now_iso(),
        }
        request_json("POST", f"{base}/api/open/deployment-sync-records", payload=sync_payload, token=token)

        if not silent:
            state.append_event(
                kind="client_applied" if changed else "client_noop",
                client=client.client_id,
                deployment_key=dep_key,
                config=config_code,
                revision=revision,
                status="success",
            )
        applied.append(f"{config_code}@{revision}")

    client.last_error = None
    return "ok: " + ", ".join(applied) if applied else "ok"


def _client_do_heartbeat(client: ClientState, state: DemoState, silent: bool = False) -> str:
    bs = client.last_bootstrap
    if not bs:
        return "not bootstrapped"
    dep_key = bs["deployment_key"]
    token = client.token or bs.get("token") or state.get_token(dep_key)
    if not token:
        return "no token"
    base = bs["config_center_base_url"].rstrip("/")

    for config_code, revision in client.last_revisions.items():
        hb_payload = {
            "project": bs["project"],
            "environment": bs["environment"],
            "deployment_key": dep_key,
            "config": config_code,
            "metadata": {
                "client": client.client_id,
                "sn": client.sn,
                "version": "demo-0.2.0",
                "applied_revision": revision,
            },
            "reported_at": now_iso(),
        }
        request_json("POST", f"{base}/api/open/heartbeats", payload=hb_payload, token=token)
        if not silent:
            state.append_event(
                kind="heartbeat",
                client=client.client_id,
                deployment_key=dep_key,
                config=config_code,
                revision=revision,
            )
    return "ok"


# ── Config-center cache polling ───────────────────────────────────────────────

def _admin_login(state: DemoState) -> bool:
    url = f"{state.config_center_url}/api/auth/login"
    username = state.run.get("admin_username", "admin")
    password = state.run.get("admin_password", "admin123456")
    status, body = request_json_with_opener(
        state.admin_opener,
        "POST",
        url,
        payload={"username": username, "password": password},
    )
    if status == 200:
        state.admin_session_ready = True
        return True
    state.admin_session_ready = False
    print(f"[control] admin login failed: {status} {body}", flush=True)
    return False


def _cc_fetch(state: DemoState, path: str) -> list[dict[str, Any]]:
    if not state.admin_session_ready:
        return []
    url = f"{state.config_center_url}{path}"
    status, body = request_json_with_opener(state.admin_opener, "GET", url)
    if status == 401:
        state.admin_session_ready = False
        if _admin_login(state):
            status, body = request_json_with_opener(state.admin_opener, "GET", url)
        else:
            return []
    if status == 200:
        return body.get("items", body) if isinstance(body, dict) else body
    return []


def _cc_poll_loop(state: DemoState, stop: threading.Event) -> None:
    for _attempt in range(5):
        if _admin_login(state):
            break
        stop.wait(3)
    print(f"[control] config-center cache polling started (project_id={state.project_id})", flush=True)
    while not stop.wait(3):
        if state.project_id is None:
            continue
        pid = state.project_id
        try:
            instances = _cc_fetch(state, f"/api/deployment-instances?project_id={pid}&page_size=100")
            releases = _cc_fetch(state, f"/api/releases?project_id={pid}")
            heartbeats = _cc_fetch(state, f"/api/deployment-heartbeats?project_id={pid}")
            sync_records = _cc_fetch(state, f"/api/deployment-sync-records?project_id={pid}")
            with state._lock:
                state.cc_instances = instances
                state.cc_releases = releases
                state.cc_heartbeats = heartbeats
                state.cc_sync_records = sync_records
        except Exception as exc:  # noqa: BLE001
            print(f"[control] cc poll error: {exc}", flush=True)


# ── Business backend HTTP handler ─────────────────────────────────────────────

def make_backend_handler(shared_state: DemoState, backend_name: str) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        server_version = "CoffeeDemoBackend/0.2"

        def log_message(self, fmt: str, *args: Any) -> None:
            sys.stdout.write(f"[{backend_name}] {fmt % args}\n")
            sys.stdout.flush()

        def write_json(self, code: int, payload: dict[str, Any]) -> None:
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:
            parsed = urllib.parse.urlparse(self.path)
            if parsed.path != "/api/bootstrap/config-center":
                self.write_json(404, {"code": "not_found", "message": "route not found"})
                return
            query = urllib.parse.parse_qs(parsed.query)
            sn = query.get("sn", [""])[0]
            deployment_key = shared_state.get_deployment_key(backend_name, sn)
            if not deployment_key:
                shared_state.append_event(kind="bootstrap_failed", backend=backend_name, sn=sn, message="SN not bound")
                self.write_json(404, {"code": "sn_not_bound", "message": "SN is not bound"})
                return
            token = shared_state.get_token(deployment_key)
            if not token:
                self.write_json(500, {"code": "demo_token_missing", "message": f"token missing for {deployment_key}"})
                return
            payload = {
                "config_center_base_url": shared_state.config_center_url,
                "project": shared_state.run["project_code"],
                "environment": "prod",
                "deployment_key": deployment_key,
                "token": token,
                "configs": ["coffee-main", "store-flags"],
            }
            shared_state.append_event(kind="bootstrap", backend=backend_name, sn=sn, deployment_key=deployment_key)
            self.write_json(200, payload)

    return Handler


# ── Control API HTTP handler ──────────────────────────────────────────────────

class ControlHandler(BaseHTTPRequestHandler):
    shared_state: DemoState  # injected as class attribute before use

    server_version = "CoffeeControlAPI/0.2"

    def log_message(self, fmt: str, *args: Any) -> None:
        sys.stdout.write(f"[control] {fmt % args}\n")
        sys.stdout.flush()

    def write_json(self, code: int, payload: Any) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self) -> None:
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def read_body(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length).decode("utf-8") if length else "{}"
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {}

    def path_parts(self) -> list[str]:
        return urllib.parse.urlparse(self.path).path.strip("/").split("/")

    def do_GET(self) -> None:
        parts = self.path_parts()
        if parts == ["api", "demo", "state"]:
            self.write_json(200, self.shared_state.snapshot())
        else:
            self.write_json(404, {"message": "not found"})

    def do_POST(self) -> None:
        parts = self.path_parts()
        state = self.shared_state
        body = self.read_body()

        try:
            if parts == ["api", "demo", "events", "clear"]:
                state.clear_events()
                self.write_json(200, {"ok": True})

            elif parts == ["api", "demo", "clients"]:
                client_id = body.get("client_id", "").strip()
                backend_name = body.get("backend_name", "backend-a")
                sn = body.get("sn", "").strip()
                if not client_id or not sn:
                    self.write_json(400, {"message": "client_id and sn are required"})
                    return
                state.ensure_client(client_id, backend_name, sn)
                self.write_json(200, {"ok": True})

            elif len(parts) >= 4 and parts[:3] == ["api", "demo", "clients"]:
                client_id = parts[3]
                client = state.get_client(client_id)
                if client is None:
                    self.write_json(404, {"message": f"client {client_id!r} not found"})
                    return

                action = parts[4] if len(parts) > 4 else ""
                sub = parts[5] if len(parts) > 5 else ""

                if action == "bootstrap":
                    msg = _client_do_bootstrap(client, state)
                    self.write_json(200 if msg == "ok" else 502, {"ok": msg == "ok", "message": msg})

                elif action == "pull":
                    msg = _client_do_pull(client, state)
                    self.write_json(200 if msg.startswith("ok") else 502, {"ok": msg.startswith("ok"), "message": msg})

                elif action == "apply":
                    msg = _client_do_apply(client, state)
                    self.write_json(200 if msg.startswith("ok") else 502, {"ok": msg.startswith("ok"), "message": msg})

                elif action == "heartbeat":
                    msg = _client_do_heartbeat(client, state)
                    self.write_json(200 if msg == "ok" else 502, {"ok": msg == "ok", "message": msg})

                elif action == "token":
                    token = body.get("token", "").strip()
                    if not token:
                        self.write_json(400, {"message": "token is required"})
                        return
                    client.token = token
                    client.status = "idle" if client.status == "error" else client.status
                    state.append_event(kind="token_set", client=client_id)
                    self.write_json(200, {"ok": True})

                elif action == "poll" and sub == "start":
                    interval = int(body.get("interval", 5))
                    client.start_poll(state, interval=interval)
                    self.write_json(200, {"ok": True})

                elif action == "poll" and sub == "pause":
                    client.pause_poll()
                    self.write_json(200, {"ok": True})

                else:
                    self.write_json(404, {"message": f"unknown client action: {action}/{sub}"})

            elif len(parts) == 5 and parts[:2] == ["api", "demo"] and parts[2] == "backends" and parts[4] == "sn-bindings":
                backend = parts[3]
                sn = body.get("sn", "").strip()
                dep_key = body.get("deployment_key", "").strip()
                if not sn or not dep_key:
                    self.write_json(400, {"message": "sn and deployment_key are required"})
                    return
                state.bind_sn(backend, sn, dep_key)
                state.append_event(kind="sn_bound", backend=backend, sn=sn, deployment_key=dep_key)
                self.write_json(200, {"ok": True})

            else:
                self.write_json(404, {"message": "not found"})

        except Exception as exc:  # noqa: BLE001
            traceback.print_exc()
            self.write_json(500, {"message": str(exc)})

    def do_DELETE(self) -> None:
        parts = self.path_parts()
        state = self.shared_state
        # DELETE /api/demo/backends/:backend/sn-bindings/:sn
        if (
            len(parts) == 6
            and parts[:2] == ["api", "demo"]
            and parts[2] == "backends"
            and parts[4] == "sn-bindings"
        ):
            backend = parts[3]
            sn = urllib.parse.unquote(parts[5])
            state.unbind_sn(backend, sn)
            state.append_event(kind="sn_unbound", backend=backend, sn=sn)
            self.write_json(200, {"ok": True})
        else:
            self.write_json(404, {"message": "not found"})


# ── Main entrypoint ───────────────────────────────────────────────────────────

def run_serve() -> None:
    run = load_run()
    shared_state = DemoState(run)
    ControlHandler.shared_state = shared_state  # type: ignore[attr-defined]

    servers: list[ThreadingHTTPServer] = []
    stop_event = threading.Event()

    def shutdown(_signum: int, _frame: Any) -> None:
        stop_event.set()
        for s in servers:
            threading.Thread(target=s.shutdown, daemon=True).start()

    signal.signal(signal.SIGTERM, shutdown)
    signal.signal(signal.SIGINT, shutdown)

    for backend_name, address in BACKENDS.items():
        handler_cls = make_backend_handler(shared_state, backend_name)
        srv = ThreadingHTTPServer(address, handler_cls)
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        servers.append(srv)
        print(f"[{backend_name}] bootstrap: http://{address[0]}:{address[1]}/api/bootstrap/config-center", flush=True)

    ctrl_srv = ThreadingHTTPServer(CONTROL_ADDR, ControlHandler)
    threading.Thread(target=ctrl_srv.serve_forever, daemon=True).start()
    servers.append(ctrl_srv)
    print(f"[control] demo API: http://{CONTROL_ADDR[0]}:{CONTROL_ADDR[1]}/api/demo/state", flush=True)

    cc_poll_thread = threading.Thread(
        target=_cc_poll_loop, args=(shared_state, stop_event), daemon=True
    )
    cc_poll_thread.start()

    print("[demo] all services running. Ctrl+C to stop.", flush=True)
    stop_event.wait()
    print("[demo] shutting down…", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "mode",
        nargs="?",
        default="serve",
        choices=["serve", "backend"],
        help="serve (default): run all demo services; backend: backend-only legacy mode",
    )
    args = parser.parse_args()

    if args.mode == "serve":
        run_serve()
    elif args.mode == "backend":
        # Legacy: backends only, no control API
        run = load_run()
        shared = DemoState(run)
        ControlHandler.shared_state = shared  # type: ignore[attr-defined]
        for name, addr in BACKENDS.items():
            h = make_backend_handler(shared, name)
            s = ThreadingHTTPServer(addr, h)
            threading.Thread(target=s.serve_forever, daemon=True).start()
            print(f"[{name}] http://{addr[0]}:{addr[1]}", flush=True)
        signal.pause()


if __name__ == "__main__":
    main()
