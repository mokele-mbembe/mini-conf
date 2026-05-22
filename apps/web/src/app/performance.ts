import type { Router } from "vue-router";

type PerfEntryKind = "api" | "route";

interface BasePerfEntry {
  kind: PerfEntryKind;
  name: string;
  duration_ms: number;
  started_at: number;
  ended_at: number;
}

export interface ApiPerfEntry extends BasePerfEntry {
  kind: "api";
  method: string;
  path: string;
  status: number;
  ok: boolean;
}

export interface RoutePerfEntry extends BasePerfEntry {
  kind: "route";
  from: string;
  to: string;
  failure?: string;
}

export type MiniConfPerfEntry = ApiPerfEntry | RoutePerfEntry;

interface MiniConfPerfStore {
  entries: MiniConfPerfEntry[];
  clear(): void;
  snapshot(): MiniConfPerfEntry[];
}

declare global {
  interface Window {
    __MINI_CONF_PERF__?: MiniConfPerfStore;
  }
}

const MAX_ENTRIES = 300;
const routeStarts = new Map<string, number>();

export function setupRouterPerformance(router: Router): void {
  router.beforeEach((to, from) => {
    routeStarts.set(routeKey(to.fullPath, from.fullPath), now());
    return true;
  });

  router.afterEach((to, from, failure) => {
    const key = routeKey(to.fullPath, from.fullPath);
    const startedAt = routeStarts.get(key) ?? now();
    routeStarts.delete(key);
    const endedAt = now();

    recordPerfEntry({
      kind: "route",
      name: `${from.fullPath || "(start)"} -> ${to.fullPath}`,
      from: from.fullPath,
      to: to.fullPath,
      failure: failure ? String(failure.type) : undefined,
      duration_ms: roundDuration(endedAt - startedAt),
      started_at: startedAt,
      ended_at: endedAt,
    });
  });
}

export function recordApiTiming(params: {
  method: string;
  path: string;
  status: number;
  ok: boolean;
  startedAt: number;
  endedAt: number;
}): void {
  recordPerfEntry({
    kind: "api",
    name: `${params.method.toUpperCase()} ${params.path}`,
    method: params.method.toUpperCase(),
    path: params.path,
    status: params.status,
    ok: params.ok,
    duration_ms: roundDuration(params.endedAt - params.startedAt),
    started_at: params.startedAt,
    ended_at: params.endedAt,
  });
}

export function now(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function recordPerfEntry(entry: MiniConfPerfEntry): void {
  if (typeof window === "undefined") {
    return;
  }

  const store = ensureStore();
  store.entries.push(entry);
  if (store.entries.length > MAX_ENTRIES) {
    store.entries.splice(0, store.entries.length - MAX_ENTRIES);
  }
}

function ensureStore(): MiniConfPerfStore {
  if (window.__MINI_CONF_PERF__) {
    return window.__MINI_CONF_PERF__;
  }

  const store: MiniConfPerfStore = {
    entries: [],
    clear() {
      this.entries.splice(0, this.entries.length);
    },
    snapshot() {
      return [...this.entries];
    },
  };
  window.__MINI_CONF_PERF__ = store;
  return store;
}

function routeKey(to: string, from: string): string {
  return `${from} -> ${to}`;
}

function roundDuration(value: number): number {
  return Math.round(value * 100) / 100;
}
