use crate::state::AppState;
use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};

const HTTP_DURATION_BUCKETS_MS: [u64; 11] =
    [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, u64::MAX];
const BUSINESS_DURATION_BUCKETS_MS: [u64; 11] =
    [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, u64::MAX];
const BUSINESS_OBSERVATION_BUCKETS: [u64; 11] = [0, 1, 2, 5, 10, 20, 50, 100, 250, 500, u64::MAX];

#[derive(Debug)]
pub struct AppMetrics {
    started_at: Instant,
    http: Mutex<BTreeMap<HttpMetricKey, HttpMetricValue>>,
    business_duration: Mutex<BTreeMap<BusinessDurationMetricKey, BusinessDurationMetricValue>>,
    business_observation:
        Mutex<BTreeMap<BusinessObservationMetricKey, BusinessObservationMetricValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HttpMetricKey {
    method: String,
    route: String,
    status: u16,
}

#[derive(Debug, Clone)]
struct HttpMetricValue {
    count: u64,
    sum_ms: f64,
    buckets: [u64; HTTP_DURATION_BUCKETS_MS.len()],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BusinessDurationMetricKey {
    operation: String,
    outcome: String,
}

#[derive(Debug, Clone)]
struct BusinessDurationMetricValue {
    count: u64,
    sum_ms: f64,
    buckets: [u64; BUSINESS_DURATION_BUCKETS_MS.len()],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BusinessObservationMetricKey {
    operation: String,
    metric: String,
}

#[derive(Debug, Clone)]
struct BusinessObservationMetricValue {
    count: u64,
    sum: f64,
    buckets: [u64; BUSINESS_OBSERVATION_BUCKETS.len()],
}

impl AppMetrics {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            http: Mutex::new(BTreeMap::new()),
            business_duration: Mutex::new(BTreeMap::new()),
            business_observation: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn record_http_request(
        &self,
        method: &str,
        route: &str,
        status: StatusCode,
        duration: Duration,
    ) {
        let key = HttpMetricKey {
            method: method.to_owned(),
            route: route.to_owned(),
            status: status.as_u16(),
        };
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let bucket_index = HTTP_DURATION_BUCKETS_MS
            .iter()
            .position(|bucket| duration_ms <= *bucket as f64)
            .unwrap_or(HTTP_DURATION_BUCKETS_MS.len() - 1);

        let mut http = self.http.lock().unwrap_or_else(|error| error.into_inner());
        let value = http.entry(key).or_insert_with(|| HttpMetricValue {
            count: 0,
            sum_ms: 0.0,
            buckets: [0; HTTP_DURATION_BUCKETS_MS.len()],
        });
        value.count += 1;
        value.sum_ms += duration_ms;
        value.buckets[bucket_index] += 1;
    }

    pub fn record_business_operation(&self, operation: &str, outcome: &str, duration: Duration) {
        let key = BusinessDurationMetricKey {
            operation: operation.to_owned(),
            outcome: outcome.to_owned(),
        };
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let bucket_index = bucket_index(&BUSINESS_DURATION_BUCKETS_MS, duration_ms);

        let mut values = self
            .business_duration
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let value = values
            .entry(key)
            .or_insert_with(|| BusinessDurationMetricValue {
                count: 0,
                sum_ms: 0.0,
                buckets: [0; BUSINESS_DURATION_BUCKETS_MS.len()],
            });
        value.count += 1;
        value.sum_ms += duration_ms;
        value.buckets[bucket_index] += 1;
    }

    pub fn record_business_observation(&self, operation: &str, metric: &str, observed: f64) {
        let key = BusinessObservationMetricKey {
            operation: operation.to_owned(),
            metric: metric.to_owned(),
        };
        let bucket_index = bucket_index(&BUSINESS_OBSERVATION_BUCKETS, observed);

        let mut values = self
            .business_observation
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let value = values
            .entry(key)
            .or_insert_with(|| BusinessObservationMetricValue {
                count: 0,
                sum: 0.0,
                buckets: [0; BUSINESS_OBSERVATION_BUCKETS.len()],
            });
        value.count += 1;
        value.sum += observed;
        value.buckets[bucket_index] += 1;
    }

    pub fn render_prometheus(&self, db_pool: Option<&PgPool>) -> String {
        let mut output = String::new();
        output.push_str("# HELP mini_conf_process_uptime_seconds Process uptime in seconds.\n");
        output.push_str("# TYPE mini_conf_process_uptime_seconds gauge\n");
        output.push_str(&format!(
            "mini_conf_process_uptime_seconds {:.3}\n",
            self.started_at.elapsed().as_secs_f64()
        ));
        output.push('\n');
        output.push_str(
            "# HELP mini_conf_http_request_duration_ms HTTP request duration in milliseconds.\n",
        );
        output.push_str("# TYPE mini_conf_http_request_duration_ms histogram\n");

        let http = self.http.lock().unwrap_or_else(|error| error.into_inner());
        for (key, value) in http.iter() {
            let mut cumulative = 0_u64;
            for (index, upper_bound) in HTTP_DURATION_BUCKETS_MS.iter().enumerate() {
                cumulative += value.buckets[index];
                output.push_str(&format!(
                    "mini_conf_http_request_duration_ms_bucket{{method=\"{}\",route=\"{}\",status=\"{}\",le=\"{}\"}} {}\n",
                    escape_label(&key.method),
                    escape_label(&key.route),
                    key.status,
                    bucket_label(*upper_bound),
                    cumulative
                ));
            }
            output.push_str(&format!(
                "mini_conf_http_request_duration_ms_count{{method=\"{}\",route=\"{}\",status=\"{}\"}} {}\n",
                escape_label(&key.method),
                escape_label(&key.route),
                key.status,
                value.count
            ));
            output.push_str(&format!(
                "mini_conf_http_request_duration_ms_sum{{method=\"{}\",route=\"{}\",status=\"{}\"}} {:.3}\n",
                escape_label(&key.method),
                escape_label(&key.route),
                key.status,
                value.sum_ms
            ));
        }

        output.push('\n');
        output.push_str("# HELP mini_conf_business_operation_duration_ms Business operation duration in milliseconds.\n");
        output.push_str("# TYPE mini_conf_business_operation_duration_ms histogram\n");
        let business_duration = self
            .business_duration
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (key, value) in business_duration.iter() {
            let mut cumulative = 0_u64;
            for (index, upper_bound) in BUSINESS_DURATION_BUCKETS_MS.iter().enumerate() {
                cumulative += value.buckets[index];
                output.push_str(&format!(
                    "mini_conf_business_operation_duration_ms_bucket{{operation=\"{}\",outcome=\"{}\",le=\"{}\"}} {}\n",
                    escape_label(&key.operation),
                    escape_label(&key.outcome),
                    bucket_label(*upper_bound),
                    cumulative
                ));
            }
            output.push_str(&format!(
                "mini_conf_business_operation_duration_ms_count{{operation=\"{}\",outcome=\"{}\"}} {}\n",
                escape_label(&key.operation),
                escape_label(&key.outcome),
                value.count
            ));
            output.push_str(&format!(
                "mini_conf_business_operation_duration_ms_sum{{operation=\"{}\",outcome=\"{}\"}} {:.3}\n",
                escape_label(&key.operation),
                escape_label(&key.outcome),
                value.sum_ms
            ));
        }

        output.push('\n');
        output.push_str(
            "# HELP mini_conf_business_observation Business operation observed values.\n",
        );
        output.push_str("# TYPE mini_conf_business_observation histogram\n");
        let business_observation = self
            .business_observation
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (key, value) in business_observation.iter() {
            let mut cumulative = 0_u64;
            for (index, upper_bound) in BUSINESS_OBSERVATION_BUCKETS.iter().enumerate() {
                cumulative += value.buckets[index];
                output.push_str(&format!(
                    "mini_conf_business_observation_bucket{{operation=\"{}\",metric=\"{}\",le=\"{}\"}} {}\n",
                    escape_label(&key.operation),
                    escape_label(&key.metric),
                    bucket_label(*upper_bound),
                    cumulative
                ));
            }
            output.push_str(&format!(
                "mini_conf_business_observation_count{{operation=\"{}\",metric=\"{}\"}} {}\n",
                escape_label(&key.operation),
                escape_label(&key.metric),
                value.count
            ));
            output.push_str(&format!(
                "mini_conf_business_observation_sum{{operation=\"{}\",metric=\"{}\"}} {:.3}\n",
                escape_label(&key.operation),
                escape_label(&key.metric),
                value.sum
            ));
        }

        output.push('\n');
        output.push_str("# HELP mini_conf_db_pool_connected Whether a PostgreSQL pool is configured and connected.\n");
        output.push_str("# TYPE mini_conf_db_pool_connected gauge\n");
        output.push_str(&format!(
            "mini_conf_db_pool_connected {}\n",
            i32::from(db_pool.is_some())
        ));
        output.push_str(
            "# HELP mini_conf_db_pool_connections Current PostgreSQL pool connections.\n",
        );
        output.push_str("# TYPE mini_conf_db_pool_connections gauge\n");
        if let Some(pool) = db_pool {
            output.push_str(&format!(
                "mini_conf_db_pool_connections{{state=\"total\"}} {}\n",
                pool.size()
            ));
            output.push_str(&format!(
                "mini_conf_db_pool_connections{{state=\"idle\"}} {}\n",
                pool.num_idle()
            ));
        } else {
            output.push_str("mini_conf_db_pool_connections{state=\"total\"} 0\n");
            output.push_str("mini_conf_db_pool_connections{state=\"idle\"} 0\n");
        }

        output
    }
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(get_metrics))
}

async fn get_metrics(State(state): State<AppState>) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        state.metrics().render_prometheus(state.db_pool()),
    )
        .into_response()
}

fn bucket_label(value: u64) -> String {
    if value == u64::MAX {
        "+Inf".to_owned()
    } else {
        value.to_string()
    }
}

fn bucket_index(buckets: &[u64], value: f64) -> usize {
    buckets
        .iter()
        .position(|bucket| value <= *bucket as f64)
        .unwrap_or(buckets.len() - 1)
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', r"\\")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
}

#[cfg(test)]
mod tests {
    use super::AppMetrics;
    use axum::http::StatusCode;
    use std::time::Duration;

    #[test]
    fn renders_http_duration_metrics() {
        let metrics = AppMetrics::new();
        metrics.record_http_request(
            "GET",
            "/api/healthz",
            StatusCode::OK,
            Duration::from_millis(7),
        );

        let rendered = metrics.render_prometheus(None);

        assert!(rendered.contains("mini_conf_process_uptime_seconds"));
        assert!(rendered.contains("mini_conf_http_request_duration_ms_bucket"));
        assert!(rendered.contains("method=\"GET\""));
        assert!(rendered.contains("route=\"/api/healthz\""));
        assert!(rendered.contains("status=\"200\""));
        assert!(rendered.contains("mini_conf_http_request_duration_ms_count"));
        assert!(rendered.contains("mini_conf_http_request_duration_ms_sum"));
        assert!(rendered.contains("mini_conf_db_pool_connected 0"));
        assert!(rendered.contains("mini_conf_db_pool_connections{state=\"total\"} 0"));
    }

    #[test]
    fn renders_business_metrics() {
        let metrics = AppMetrics::new();
        metrics.record_business_operation("draft_save", "ok", Duration::from_millis(11));
        metrics.record_business_observation("open_config_bundle", "config_count", 3.0);

        let rendered = metrics.render_prometheus(None);

        assert!(rendered.contains("mini_conf_business_operation_duration_ms_bucket"));
        assert!(rendered.contains("operation=\"draft_save\""));
        assert!(rendered.contains("outcome=\"ok\""));
        assert!(rendered.contains("mini_conf_business_observation_bucket"));
        assert!(rendered.contains("operation=\"open_config_bundle\""));
        assert!(rendered.contains("metric=\"config_count\""));
    }
}
