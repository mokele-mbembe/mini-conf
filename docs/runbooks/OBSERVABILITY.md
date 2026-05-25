# Observability Runbook

## 1. Scope

This runbook covers the first production observability baseline for `mini-conf`:

- scrape the application `/metrics` endpoint with Prometheus
- load the bundled alert rules
- import the bundled Grafana overview dashboard
- use the GitHub `Perf` workflow for weekly S/M gates and nightly L dataset trend sampling

The application exposes Prometheus text format at:

```text
GET /metrics
```

## 2. Prometheus

Example config:

- `deploy/observability/prometheus.yml`

Alert rules:

- `deploy/observability/rules/mini-conf-alerts.yml`

The example target assumes the application listens on `127.0.0.1:8080`:

```yaml
scrape_configs:
  - job_name: mini-conf
    metrics_path: /metrics
    static_configs:
      - targets:
          - 127.0.0.1:8080
```

For production, replace the target with the internal address that Prometheus can reach. Do not scrape through the public CDN or browser-facing HTTPS path unless that is the only available network boundary.

## 3. Grafana

Provisioning files:

- `deploy/observability/grafana/provisioning/datasources/prometheus.yml`
- `deploy/observability/grafana/provisioning/dashboards/mini-conf.yml`

Dashboard:

- `deploy/observability/grafana/dashboards/mini-conf-overview.json`

When using Grafana file provisioning, mount the dashboard JSON into:

```text
/etc/grafana/provisioning/dashboards/mini-conf/
```

The dashboard expects a Prometheus datasource with UID `Prometheus`.

## 4. Initial Alerts

The bundled rule file starts with conservative runtime alerts:

- scrape target down
- database pool disconnected
- HTTP p95 above 250ms
- Open API p95 above 100ms
- HTTP 5xx rate above 1%
- business operation p95 above 250ms
- PostgreSQL pool utilization above 80%

These are first-pass operational thresholds. After production traffic is available, tune them using 7 to 14 days of real traffic and keep CI thresholds separate from paging thresholds.

## 5. GitHub Perf Trend

`.github/workflows/perf.yml` now has two scheduled modes:

- weekly full Perf workflow: S/M backend smoke, web perf smoke, bundle budget
- nightly L dataset trend: backend `PERF_SMOKE_DATASET=L` collection without CI gate enforcement

Manual dispatch keeps the normal Perf jobs and baseline calibration. To also run the L trend job manually, enable the `run_l_trend` input.

The L trend job uploads:

- `target/perf/trend-l/smoke-L.json`
- `target/perf/trend-l/db-slow-queries-L.json`
- `target/perf/trend-l/summary-L.md`

Use the uploaded `measured_ms`, route/API p95, RSS, and slow-query report as trend signals. Do not promote the L trend threshold into CI until several samples are stable on the same runner class.
