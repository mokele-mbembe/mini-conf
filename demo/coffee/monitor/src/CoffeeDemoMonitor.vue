<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import type {
  DemoState,
  ClientState,
  CcInstance,
  CcRelease,
  CcHeartbeat,
} from "./api";
import {
  fetchState,
  clientBootstrap,
  clientPull,
  clientApply,
  clientHeartbeat,
  clientStartPoll,
  clientPausePoll,
  clientSetToken,
  clientCreate,
  bindSn,
  unbindSn,
  clearEvents,
} from "./api";

// ── State ────────────────────────────────────────────────────────────────
const state = ref<DemoState | null>(null);
const loadError = ref<string | null>(null);
const actionErrors = ref<Record<string, string>>({});
const actionBusy = ref<Record<string, boolean>>({});

// Token paste inputs keyed by client_id
const tokenInputs = ref<Record<string, string>>({});

// New client form
const newClientId = ref("");
const newClientBackend = ref("backend-a");
const newClientSn = ref("");

// New SN bind form per backend
const bindSnForm = ref<Record<string, { sn: string; key: string }>>({});

// ── Auto-refresh ─────────────────────────────────────────────────────────
let pollTimer: ReturnType<typeof setInterval> | null = null;
const autoRefresh = ref(true);

async function load() {
  try {
    state.value = await fetchState();
    loadError.value = null;
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : "Failed to load state";
  }
}

function startAutoRefresh() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = setInterval(() => {
    if (autoRefresh.value) load();
  }, 2000);
}

onMounted(() => {
  load();
  startAutoRefresh();
});
onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
});

// ── Action helper ─────────────────────────────────────────────────────────
async function act(key: string, fn: () => Promise<unknown>) {
  actionBusy.value[key] = true;
  actionErrors.value[key] = "";
  try {
    await fn();
    await load();
  } catch (e) {
    actionErrors.value[key] = e instanceof Error ? e.message : String(e);
  } finally {
    actionBusy.value[key] = false;
  }
}

// ── Client actions ────────────────────────────────────────────────────────
const busy = (k: string) => actionBusy.value[k] ?? false;
const err = (k: string) => actionErrors.value[k] ?? "";

function doBootstrap(id: string) {
  act(`bootstrap:${id}`, () => clientBootstrap(id));
}
function doPull(id: string) {
  act(`pull:${id}`, () => clientPull(id));
}
function doApply(id: string) {
  act(`apply:${id}`, () => clientApply(id));
}
function doHeartbeat(id: string) {
  act(`hb:${id}`, () => clientHeartbeat(id));
}

function doStartPoll(id: string) {
  act(`poll:${id}`, () => clientStartPoll(id));
}
function doPausePoll(id: string) {
  act(`poll:${id}`, () => clientPausePoll(id));
}

function doSetToken(id: string) {
  const token = tokenInputs.value[id]?.trim();
  if (!token) return;
  act(`token:${id}`, () =>
    clientSetToken(id, token).then(() => {
      tokenInputs.value[id] = "";
    }),
  );
}

function doCreateClient() {
  const id = newClientId.value.trim();
  const backend = newClientBackend.value;
  const sn = newClientSn.value.trim();
  if (!id || !sn) return;
  act("newclient", () =>
    clientCreate(id, backend, sn).then(() => {
      newClientId.value = "";
      newClientSn.value = "";
    }),
  );
}

// ── SN binding actions ────────────────────────────────────────────────────
function doBindSn(backend: string) {
  const form = bindSnForm.value[backend];
  if (!form?.sn || !form?.key) return;
  act(`bind:${backend}:${form.sn}`, () =>
    bindSn(backend, form.sn, form.key).then(() => {
      form.sn = "";
      form.key = "";
    }),
  );
}
function doUnbindSn(backend: string, sn: string) {
  act(`unbind:${backend}:${sn}`, () => unbindSn(backend, sn));
}

function initBindForm(backend: string) {
  if (!bindSnForm.value[backend])
    bindSnForm.value[backend] = { sn: "", key: "" };
  return bindSnForm.value[backend];
}

// ── Computed: revision summary per instance ──────────────────────────────
const COFFEE_MAIN = "coffee-main";

const latestReleaseByInstance = computed(() => {
  const map = new Map<number, CcRelease>();
  for (const r of state.value?.cc_releases ?? []) {
    const existing = map.get(r.deployment_instance_id);
    if (!existing || r.published_at > existing.published_at) {
      map.set(r.deployment_instance_id, r);
    }
  }
  return map;
});

const latestCoffeeMainHeartbeat = computed(() => {
  const map = new Map<number, CcHeartbeat>();
  for (const h of state.value?.cc_heartbeats ?? []) {
    if (h.config !== COFFEE_MAIN) continue;
    const existing = map.get(h.deployment_instance_id);
    if (!existing || h.reported_at > existing.reported_at) {
      map.set(h.deployment_instance_id, h);
    }
  }
  return map;
});

interface InstanceRow {
  inst: CcInstance;
  backend: string;
  sn: string;
  latestRev: string | null;
  appliedRev: string | null;
  lastHb: string | null;
  stale: boolean;
}

// Infer SN routing from SN bindings
const instanceToSn = computed(() => {
  const map: Record<string, { backend: string; sn: string }> = {};
  for (const [backend, bindings] of Object.entries(
    state.value?.sn_bindings ?? {},
  )) {
    for (const [sn, depKey] of Object.entries(bindings)) {
      map[depKey] = { backend, sn };
    }
  }
  return map;
});

const instanceRows = computed<InstanceRow[]>(() => {
  return (state.value?.cc_instances ?? [])
    .filter((i) => !i.is_template && !i.deleted_at)
    .map((i) => {
      const routing = instanceToSn.value[i.deployment_key] ?? {
        backend: "—",
        sn: "—",
      };
      const rel = latestReleaseByInstance.value.get(i.id);
      const hb = latestCoffeeMainHeartbeat.value.get(i.id);
      const meta = hb?.metadata as
        | { applied_revision?: string }
        | null
        | undefined;
      const latestRev = rel?.revision ?? null;
      const appliedRev = meta?.applied_revision ?? null;
      return {
        inst: i,
        backend: routing.backend,
        sn: routing.sn,
        latestRev,
        appliedRev,
        lastHb: hb?.reported_at ?? null,
        stale: !!(latestRev && appliedRev && latestRev !== appliedRev),
      };
    });
});

// ── Timeline ──────────────────────────────────────────────────────────────
const timelineEvents = computed(() => {
  return [...(state.value?.events ?? [])].reverse().slice(0, 100);
});

// ── Helpers ───────────────────────────────────────────────────────────────
function ts(s: string) {
  try {
    return new Date(s).toLocaleTimeString();
  } catch {
    return s;
  }
}

function clientStatusBadge(s: ClientState["status"]) {
  const map: Record<string, string> = {
    idle: "badge-gray",
    bootstrapped: "badge-blue",
    polling: "badge-green",
    paused: "badge-yellow",
    error: "badge-red",
  };
  return map[s] ?? "badge-gray";
}

function instanceStatusBadge(s: string) {
  if (s === "active") return "badge-green";
  if (s === "inactive") return "badge-yellow";
  return "badge-gray";
}

function eventClass(e: { kind: string }) {
  if (e.kind.includes("heartbeat")) return "heartbeat";
  if (e.kind.includes("sync") || e.kind.includes("apply")) return "sync";
  if (e.kind.includes("bootstrap")) return "bootstrap";
  if (e.kind.includes("error") || e.kind.includes("fail")) return "error";
  return "audit";
}
</script>

<template>
  <div class="monitor-root">
    <!-- ── Topbar ─────────────────────────────────────────────────── -->
    <div class="topbar">
      <span>☕</span>
      <h1>Coffee Demo Monitor</h1>
      <span v-if="state" class="mono project-code">
        {{ state.project_code }}
      </span>
      <div
        :class="['status-dot', loadError ? 'error' : '']"
        :title="loadError ?? 'Connected'"
      />
      <label
        style="
          display: flex;
          align-items: center;
          gap: 6px;
          font-size: 12px;
          cursor: pointer;
        "
      >
        <input type="checkbox" v-model="autoRefresh" /> Auto-refresh
      </label>
      <button class="btn btn-sm" @click="load">⟳ Refresh</button>
      <button class="btn btn-sm btn-danger" @click="act('clear', clearEvents)">
        Clear Events
      </button>
      <a
        href="http://127.0.0.1:5173"
        target="_blank"
        class="btn btn-sm"
        style="text-decoration: none"
        >🔧 Admin UI</a
      >
    </div>

    <div v-if="loadError" class="error-banner">⚠ {{ loadError }}</div>

    <div
      v-if="!state"
      style="color: var(--text-dim); padding: 40px; text-align: center"
    >
      Loading… (is demo-coffee-access-app.py serve running on :19010?)
    </div>

    <template v-else>
      <!-- ── Manual runbook ─────────────────────────────────────── -->
      <div class="guide">
        <div>
          <h2>Manual demo flow</h2>
          <p>
            Use Admin UI for human operations: clone, activate, edit Draft,
            preview bundle, and publish. Use this monitor to make clients
            bootstrap, pull, apply, and report heartbeat on demand.
          </p>
        </div>
        <ol>
          <li>
            Existing store: Bootstrap -> Pull Bundle -> Apply -> Heartbeat.
          </li>
          <li>
            After publishing in Admin UI: Pull Bundle -> Apply -> Heartbeat
            again.
          </li>
          <li>
            New store: bind SN, create client, paste activation token, then run
            the same client actions.
          </li>
        </ol>
      </div>

      <!-- ── Topology ─────────────────────────────────────────────── -->
      <div class="card">
        <div class="card-header">📡 Topology</div>
        <div class="card-body">
          <div class="topo">
            <div
              v-for="(bindings, backend) in state.sn_bindings"
              :key="backend"
              class="topo-row"
            >
              <div
                v-for="(depKey, sn) in bindings"
                :key="sn"
                class="topo-node topo-node-client"
              >
                <div class="label">{{ sn }}</div>
                <div class="sub">{{ backend }}</div>
              </div>
              <div class="topo-arrow">
                <div class="topo-arrow-line" />
                <div class="topo-arrow-label">bootstrap</div>
              </div>
              <div class="topo-node topo-node-backend">
                <div class="label">{{ backend }}</div>
                <div class="sub">
                  :{{ backend === "backend-a" ? 19001 : 19002 }}
                </div>
              </div>
              <div class="topo-arrow">
                <div class="topo-arrow-line" />
                <div class="topo-arrow-label">token+key</div>
              </div>
              <div class="topo-node topo-node-center">
                <div class="label">Config Center</div>
                <div class="sub">:8080</div>
              </div>
              <div class="topo-arrow">
                <div class="topo-arrow-line" />
                <div class="topo-arrow-label">bundle</div>
              </div>
              <div class="topo-node topo-node-output">
                <div class="label">Effective Config</div>
                <div class="sub">demo/generated/</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ── Instance Status ─────────────────────────────────────── -->
      <div class="card">
        <div class="card-header">🏪 Instance Status (coffee-main)</div>
        <div class="card-body" style="padding: 0">
          <table class="data-table">
            <thead>
              <tr>
                <th>Backend</th>
                <th>SN</th>
                <th>Deployment Key</th>
                <th>Status</th>
                <th>Latest Release</th>
                <th>Client Applied</th>
                <th>Last Heartbeat</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in instanceRows" :key="row.inst.id">
                <td>
                  <span class="badge badge-blue">{{ row.backend }}</span>
                </td>
                <td>
                  <span class="mono">{{ row.sn }}</span>
                </td>
                <td>
                  <span class="mono">{{ row.inst.deployment_key }}</span>
                </td>
                <td>
                  <span
                    :class="['badge', instanceStatusBadge(row.inst.status)]"
                  >
                    {{ row.inst.status }}
                  </span>
                  <span
                    v-if="row.inst.is_archived"
                    class="badge badge-gray"
                    style="margin-left: 4px"
                    >archived</span
                  >
                </td>
                <td>
                  <span v-if="row.latestRev" class="mono">{{
                    row.latestRev
                  }}</span>
                  <span v-else class="badge badge-gray">none</span>
                </td>
                <td>
                  <span
                    v-if="row.appliedRev"
                    :class="['mono', row.stale ? 'stale' : 'current']"
                  >
                    {{ row.appliedRev }}
                    <span
                      v-if="row.stale"
                      class="badge badge-yellow"
                      style="margin-left: 4px"
                      >stale</span
                    >
                  </span>
                  <span v-else class="badge badge-gray">—</span>
                </td>
                <td>
                  <span v-if="row.lastHb" style="color: var(--text-dim)">{{
                    ts(row.lastHb)
                  }}</span>
                  <span v-else class="badge badge-gray">—</span>
                </td>
              </tr>
              <tr v-if="instanceRows.length === 0">
                <td
                  colspan="7"
                  style="
                    color: var(--text-dim);
                    text-align: center;
                    padding: 20px;
                  "
                >
                  No instances — run just demo-coffee-reset
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- ── SN Bindings ─────────────────────────────────────────── -->
      <div class="card">
        <div class="card-header">🗺 SN Bindings (Business Backend)</div>
        <div class="card-body">
          <div class="sn-table">
            <div
              v-for="(bindings, backend) in state.sn_bindings"
              :key="backend"
              class="sn-backend"
            >
              <div class="sn-backend-name">{{ backend }}</div>
              <div v-for="(depKey, sn) in bindings" :key="sn" class="sn-row">
                <span class="mono sn-code">{{ sn }}</span>
                <span class="sn-dep"
                  >→ <span class="mono">{{ depKey }}</span></span
                >
                <button
                  class="btn btn-sm btn-danger"
                  :disabled="busy(`unbind:${backend}:${sn}`)"
                  @click="doUnbindSn(backend, sn)"
                >
                  Unbind
                </button>
              </div>

              <!-- Bind form -->
              <div class="bind-form">
                <input
                  v-model="initBindForm(backend).sn"
                  placeholder="SN (e.g. SN003)"
                  style="max-width: 140px"
                />
                <select v-model="initBindForm(backend).key">
                  <option value="">— select instance —</option>
                  <option
                    v-for="inst in state.cc_instances.filter(
                      (i) => !i.is_template && !i.deleted_at,
                    )"
                    :key="inst.id"
                    :value="inst.deployment_key"
                  >
                    {{ inst.deployment_key }}
                  </option>
                </select>
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="
                    busy(`bind:${backend}:${initBindForm(backend).sn}`)
                  "
                  @click="doBindSn(backend)"
                >
                  Bind
                </button>
              </div>
              <div
                v-if="err(`bind:${backend}:${initBindForm(backend).sn}`)"
                style="color: var(--red); font-size: 11px; margin-top: 4px"
              >
                {{ err(`bind:${backend}:${initBindForm(backend).sn}`) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ── Client Actions ──────────────────────────────────────── -->
      <div class="card">
        <div class="card-header">🖥 Simulated Clients</div>
        <div class="card-body">
          <div class="client-grid">
            <div
              v-for="(client, clientId) in state.clients"
              :key="clientId"
              class="client-card"
            >
              <div class="client-card-header">
                <span class="client-name">{{ clientId }}</span>
                <span :class="['badge', clientStatusBadge(client.status)]">{{
                  client.status
                }}</span>
              </div>

              <div class="client-meta">
                <div class="kv">
                  <span class="kv-label">Backend:</span>
                  <span class="mono">{{ client.backend_name }}</span>
                </div>
                <div class="kv">
                  <span class="kv-label">SN:</span>
                  <span class="mono">{{ client.sn }}</span>
                </div>
                <div v-if="client.last_bootstrap" class="kv">
                  <span class="kv-label">Dep Key:</span>
                  <span class="mono">{{
                    client.last_bootstrap.deployment_key
                  }}</span>
                </div>
                <div
                  v-if="Object.keys(client.last_revisions).length > 0"
                  class="kv"
                >
                  <span class="kv-label">Applied:</span>
                  <span class="mono">{{
                    Object.entries(client.last_revisions)
                      .map(([k, v]) => `${k}:${v}`)
                      .join(" ")
                  }}</span>
                </div>
                <div v-if="client.last_error" style="color: var(--red)">
                  {{ client.last_error }}
                </div>
              </div>

              <div class="client-actions">
                <div class="btn-group">
                  <button
                    class="btn btn-sm btn-primary"
                    :disabled="busy(`bootstrap:${clientId}`)"
                    @click="doBootstrap(clientId)"
                  >
                    Bootstrap
                  </button>
                  <button
                    class="btn btn-sm"
                    :disabled="
                      busy(`pull:${clientId}`) || !client.last_bootstrap
                    "
                    @click="doPull(clientId)"
                  >
                    Pull Bundle
                  </button>
                  <button
                    class="btn btn-sm"
                    :disabled="busy(`apply:${clientId}`) || !client.last_bundle"
                    @click="doApply(clientId)"
                  >
                    Apply
                  </button>
                  <button
                    class="btn btn-sm"
                    :disabled="busy(`hb:${clientId}`) || !client.last_bootstrap"
                    @click="doHeartbeat(clientId)"
                  >
                    Heartbeat
                  </button>
                </div>
                <div class="btn-group">
                  <button
                    v-if="client.status !== 'polling'"
                    class="btn btn-sm"
                    :disabled="
                      busy(`poll:${clientId}`) || !client.last_bootstrap
                    "
                    @click="doStartPoll(clientId)"
                  >
                    ▶ Start Polling
                  </button>
                  <button
                    v-else
                    class="btn btn-sm btn-danger"
                    :disabled="busy(`poll:${clientId}`)"
                    @click="doPausePoll(clientId)"
                  >
                    ⏸ Pause Polling
                  </button>
                </div>

                <!-- Token paste (for newly activated instances) -->
                <div class="token-panel">
                  <div class="token-title">
                    Activation token
                    <span v-if="client.token" class="badge badge-green"
                      >set</span
                    >
                    <span v-else class="badge badge-yellow">needed</span>
                  </div>
                  <div class="token-help">
                    Copy this from the Admin UI activation dialog. Seeded
                    clients already have a demo token, but you can replace it
                    here.
                  </div>
                  <div class="token-row">
                    <input
                      v-model="tokenInputs[clientId]"
                      placeholder="Paste activation token from Admin UI"
                      type="password"
                    />
                    <button
                      class="btn btn-sm btn-primary"
                      :disabled="
                        busy(`token:${clientId}`) || !tokenInputs[clientId]
                      "
                      @click="doSetToken(clientId)"
                    >
                      Set Token
                    </button>
                  </div>
                </div>

                <div
                  v-if="
                    err(`bootstrap:${clientId}`) ||
                    err(`pull:${clientId}`) ||
                    err(`apply:${clientId}`)
                  "
                  style="color: var(--red); font-size: 11px"
                >
                  {{
                    err(`bootstrap:${clientId}`) ||
                    err(`pull:${clientId}`) ||
                    err(`apply:${clientId}`)
                  }}
                </div>
              </div>
            </div>

            <!-- Add new client -->
            <div class="client-card" style="border-style: dashed; opacity: 0.7">
              <div class="client-card-header">
                <span class="client-name" style="color: var(--text-dim)"
                  >+ New Client</span
                >
              </div>
              <div class="client-meta">
                Use the deployment_key of the newly cloned instance.
              </div>
              <div class="client-actions" style="gap: 6px">
                <input
                  v-model="newClientId"
                  placeholder="deployment_key (e.g. a-prod-store-003)"
                  class="bind-form"
                  style="
                    background: var(--surface2);
                    border: 1px solid var(--border);
                    border-radius: 6px;
                    color: var(--text);
                    padding: 4px 8px;
                    font-size: 12px;
                    width: 100%;
                  "
                />
                <div style="display: flex; gap: 6px">
                  <select
                    v-model="newClientBackend"
                    style="
                      background: var(--surface2);
                      border: 1px solid var(--border);
                      border-radius: 6px;
                      color: var(--text);
                      padding: 4px 8px;
                      font-size: 12px;
                      flex: 1;
                    "
                  >
                    <option value="backend-a">backend-a</option>
                    <option value="backend-b">backend-b</option>
                  </select>
                  <input
                    v-model="newClientSn"
                    placeholder="SN003"
                    style="
                      background: var(--surface2);
                      border: 1px solid var(--border);
                      border-radius: 6px;
                      color: var(--text);
                      padding: 4px 8px;
                      font-size: 12px;
                      width: 80px;
                    "
                  />
                </div>
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="busy('newclient') || !newClientId || !newClientSn"
                  @click="doCreateClient"
                >
                  Create Client
                </button>
                <div
                  v-if="err('newclient')"
                  style="color: var(--red); font-size: 11px"
                >
                  {{ err("newclient") }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ── Effective Configs ───────────────────────────────────── -->
      <div v-if="Object.keys(state.effective_configs).length > 0" class="card">
        <div class="card-header">📄 Effective Configs (last applied)</div>
        <div class="card-body">
          <div
            v-for="(content, filename) in state.effective_configs"
            :key="filename"
            style="margin-bottom: 12px"
          >
            <div
              style="
                font-size: 11px;
                color: var(--text-dim);
                margin-bottom: 4px;
              "
            >
              {{ filename }}
            </div>
            <pre
              style="
                background: var(--surface2);
                border: 1px solid var(--border);
                border-radius: 6px;
                padding: 10px;
                font-size: 11px;
                overflow-x: auto;
                white-space: pre-wrap;
                max-height: 200px;
                overflow-y: auto;
              "
              >{{ content }}</pre
            >
          </div>
        </div>
      </div>

      <!-- ── Event Timeline ─────────────────────────────────────── -->
      <div class="card">
        <div class="card-header">
          📋 Event Timeline
          <span
            style="margin-left: auto; font-size: 11px; color: var(--text-dim)"
          >
            {{ timelineEvents.length }} events
          </span>
        </div>
        <div class="card-body" style="padding: 8px">
          <div class="timeline">
            <div v-if="timelineEvents.length === 0" class="tl-empty">
              No events yet — use the client action buttons above
            </div>
            <div
              v-for="(event, i) in timelineEvents"
              :key="i"
              :class="['tl-event', eventClass(event)]"
            >
              <span class="tl-ts">{{ ts(event.at) }}</span>
              <span class="tl-type">{{ event.kind }}</span>
              <span class="tl-label">
                <span v-if="event.client" class="mono">{{ event.client }}</span>
                <span v-if="event.deployment_key" class="mono">
                  {{ event.deployment_key }}</span
                >
                <span v-if="event.config"> / {{ event.config }}</span>
                <span v-if="event.revision">
                  @ <span class="mono">{{ event.revision }}</span></span
                >
              </span>
              <span
                v-if="event.status"
                :class="[
                  'badge',
                  event.status === 'success'
                    ? 'badge-green'
                    : event.status === 'noop'
                      ? 'badge-gray'
                      : 'badge-red',
                ]"
              >
                {{ event.status }}
              </span>
              <span v-if="event.message" class="tl-detail">{{
                event.message
              }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
