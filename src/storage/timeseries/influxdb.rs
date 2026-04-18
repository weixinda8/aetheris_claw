use crate::storage::timeseries::traits::*;
use crate::storage::timeseries::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
struct FluxResponse {
    tables: Vec<FluxTable>,
}

#[derive(Debug, Deserialize)]
struct FluxTable {
    columns: Vec<FluxColumn>,
    data: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct FluxColumn {
    name: String,
    r#type: String,
}

#[derive(Debug)]
struct TimeSeriesStatsInner {
    total_points_written: AtomicU64,
    total_points_read: AtomicU64,
    total_write_errors: AtomicU64,
    total_query_errors: AtomicU64,
    total_write_latency_ms: AtomicU64,
    total_query_latency_ms: AtomicU64,
    write_count: AtomicU64,
    query_count: AtomicU64,
}

impl Clone for TimeSeriesStatsInner {
    fn clone(&self) -> Self {
        Self {
            total_points_written: AtomicU64::new(self.total_points_written.load(Ordering::Relaxed)),
            total_points_read: AtomicU64::new(self.total_points_read.load(Ordering::Relaxed)),
            total_write_errors: AtomicU64::new(self.total_write_errors.load(Ordering::Relaxed)),
            total_query_errors: AtomicU64::new(self.total_query_errors.load(Ordering::Relaxed)),
            total_write_latency_ms: AtomicU64::new(self.total_write_latency_ms.load(Ordering::Relaxed)),
            total_query_latency_ms: AtomicU64::new(self.total_query_latency_ms.load(Ordering::Relaxed)),
            write_count: AtomicU64::new(self.write_count.load(Ordering::Relaxed)),
            query_count: AtomicU64::new(self.query_count.load(Ordering::Relaxed)),
        }
    }
}

impl Default for TimeSeriesStatsInner {
    fn default() -> Self {
        Self {
            total_points_written: AtomicU64::new(0),
            total_points_read: AtomicU64::new(0),
            total_write_errors: AtomicU64::new(0),
            total_query_errors: AtomicU64::new(0),
            total_write_latency_ms: AtomicU64::new(0),
            total_query_latency_ms: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
        }
    }
}

impl TimeSeriesStatsInner {
    fn to_stats(&self) -> TimeSeriesStats {
        let write_count = self.write_count.load(Ordering::Relaxed);
        let query_count = self.query_count.load(Ordering::Relaxed);

        TimeSeriesStats {
            total_points_written: self.total_points_written.load(Ordering::Relaxed),
            total_points_read: self.total_points_read.load(Ordering::Relaxed),
            total_write_errors: self.total_write_errors.load(Ordering::Relaxed),
            total_query_errors: self.total_query_errors.load(Ordering::Relaxed),
            average_write_latency_ms: if write_count > 0 {
                self.total_write_latency_ms.load(Ordering::Relaxed) as f64 / write_count as f64
            } else {
                0.0
            },
            average_query_latency_ms: if query_count > 0 {
                self.total_query_latency_ms.load(Ordering::Relaxed) as f64 / query_count as f64
            } else {
                0.0
            },
            database_size_bytes: 0,
            series_count: 0,
        }
    }
}

pub struct InfluxDBTimeSeries {
    config: TimeSeriesConfig,
    client: Client,
    connected: Arc<AtomicBool>,
    write_url: String,
    query_url: String,
    health_url: String,
    buckets_url: String,
    org: String,
    stats: Arc<TimeSeriesStatsInner>,
}

impl InfluxDBTimeSeries {
    pub fn new(config: TimeSeriesConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        let org = std::env::var("INFLUXDB_ORG").unwrap_or_else(|_| "aetheris".to_string());
        let base_url = format!("http://{}:{}", config.endpoint, config.port);
        let write_url = format!(
            "{}/api/v2/write?org={}&bucket={}",
            base_url, org, config.database
        );
        let query_url = format!("{}/api/v2/query?org={}", base_url, org);
        let health_url = format!("{}/health", base_url);
        let buckets_url = format!("{}/api/v2/buckets?org={}", base_url, org);

        Self {
            config,
            client,
            connected: Arc::new(AtomicBool::new(false)),
            write_url,
            query_url,
            health_url,
            buckets_url,
            org,
            stats: Arc::new(TimeSeriesStatsInner::default()),
        }
    }

    fn format_line_protocol(&self, point: &TimeSeriesPoint) -> String {
        let mut line = point.measurement.clone();

        for (key, value) in point.tags.iter() {
            line.push(',');
            line.push_str(&escape_tag_key(key));
            line.push('=');
            line.push_str(&escape_tag_value(value));
        }

        line.push(' ');

        let mut first_field = true;
        for (key, value) in point.fields.iter() {
            if !first_field {
                line.push(',');
            }
            first_field = false;
            line.push_str(&escape_field_key(key));
            line.push('=');
            line.push_str(&format_field_value(value));
        }

        line.push(' ');
        line.push_str(
            &point
                .timestamp
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .to_string(),
        );

        line
    }

    async fn retry_with_backoff<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut retries = 0;
        let max_retries = self.config.max_retries;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(crate::utils::AetherisError::External(format!(
                            "Operation failed after {} retries: {}",
                            max_retries, e
                        )));
                    }
                    tokio::time::sleep(self.config.retry_interval * retries).await;
                }
            }
        }
    }

    fn parse_flux_response(&self, json_str: &str) -> Result<Vec<TimeSeriesPoint>> {
        let mut points = Vec::new();
        let lines: Vec<&str> = json_str.lines().collect();

        let mut current_measurement = String::new();
        let mut current_tags: HashMap<String, String> = HashMap::new();
        let mut current_fields: HashMap<String, TimeSeriesValue> = HashMap::new();
        let mut current_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;

        let mut column_indices: HashMap<String, usize> = HashMap::new();
        let mut in_data = false;

        for line in lines {
            if line.is_empty() {
                if in_data && !current_fields.is_empty() {
                    if let Some(ts) = current_timestamp {
                        let point = TimeSeriesPoint {
                            measurement: current_measurement.clone(),
                            timestamp: ts,
                            tags: current_tags.clone(),
                            fields: current_fields.clone(),
                        };
                        points.push(point);
                    }
                }
                current_tags.clear();
                current_fields.clear();
                current_timestamp = None;
                in_data = false;
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                continue;
            }

            let result = parts[0];
            let _table = parts[1];
            let _start = parts.get(2);
            let _stop = parts.get(3);

            if result == "#group" || result == "#datatype" || result == "#default" {
                continue;
            }

            if result == "result" {
                column_indices.clear();
                for (i, col) in parts.iter().enumerate() {
                    column_indices.insert(col.to_string(), i);
                }
                continue;
            }

            in_data = true;

            if let Some(&idx) = column_indices.get("_measurement") {
                if let Some(m) = parts.get(idx) {
                    if !m.is_empty() && &**m != "_measurement" {
                        current_measurement = m.to_string();
                    }
                }
            }

            if let Some(&idx) = column_indices.get("_time") {
                if let Some(t) = parts.get(idx) {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(t) {
                        current_timestamp = Some(ts.with_timezone(&chrono::Utc));
                    }
                }
            }

            if let Some(&idx) = column_indices.get("_field") {
                if let Some(&value_idx) = column_indices.get("_value") {
                    if let (Some(field), Some(value)) = (parts.get(idx), parts.get(value_idx)) {
                        if !field.is_empty() && &**field != "_field" && !value.is_empty() {
                            if let Ok(int_val) = value.parse::<i64>() {
                                current_fields
                                    .insert(field.to_string(), TimeSeriesValue::Int64(int_val));
                            } else if let Ok(uint_val) = value.parse::<u64>() {
                                current_fields
                                    .insert(field.to_string(), TimeSeriesValue::UInt64(uint_val));
                            } else if let Ok(float_val) = value.parse::<f64>() {
                                current_fields
                                    .insert(field.to_string(), TimeSeriesValue::Float64(float_val));
                            } else if &**value == "true" || &**value == "false" {
                                current_fields.insert(
                                    field.to_string(),
                                    TimeSeriesValue::Boolean(&**value == "true"),
                                );
                            } else {
                                current_fields.insert(
                                    field.to_string(),
                                    TimeSeriesValue::String(value.to_string()),
                                );
                            }
                        }
                    }
                }
            }

            for (key, &idx) in &column_indices {
                if !key.starts_with('_') && key != "table" && key != "result" {
                    if let Some(value) = parts.get(idx) {
                        if !value.is_empty() {
                            current_tags.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        }

        if in_data && !current_fields.is_empty() {
            if let Some(ts) = current_timestamp {
                let point = TimeSeriesPoint {
                    measurement: current_measurement,
                    timestamp: ts,
                    tags: current_tags,
                    fields: current_fields,
                };
                points.push(point);
            }
        }

        Ok(points)
    }
}

#[async_trait]
impl TimeSeriesDatabase for InfluxDBTimeSeries {
    async fn connect(&mut self) -> Result<()> {
        tracing::debug!(
            "Connecting to InfluxDB at {}:{}",
            self.config.endpoint,
            self.config.port
        );

        let result = self
            .retry_with_backoff(|| async {
                let request = self.client.get(&self.health_url);
                let request = if let Some(token) = &self.config.token {
                    request.header("Authorization", format!("Token {}", token))
                } else {
                    request
                };

                let response = request.send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    return Err(crate::utils::AetherisError::External(format!(
                        "InfluxDB health check failed: {} - {}",
                        status, text
                    )));
                }

                Ok(())
            })
            .await;

        match result {
            Ok(_) => {
                self.connected.store(true, Ordering::Relaxed);
                tracing::info!("Successfully connected to InfluxDB");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to InfluxDB, operating in degraded mode: {}",
                    e
                );
                self.connected.store(false, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected.store(false, Ordering::Relaxed);
        tracing::info!("Disconnected from InfluxDB");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn write_point(&mut self, point: TimeSeriesPoint) -> Result<()> {
        let line = self.format_line_protocol(&point);
        self.write_lines(vec![line]).await
    }

    async fn write_points(&mut self, points: Vec<TimeSeriesPoint>) -> Result<()> {
        let lines: Vec<String> = points
            .iter()
            .map(|p| self.format_line_protocol(p))
            .collect();
        self.write_lines(lines).await
    }

    async fn query(&self, query: TimeSeriesQuery) -> Result<Vec<TimeSeriesPoint>> {
        let start_time = Instant::now();
        let result = self.do_query(query).await;
        let latency = start_time.elapsed().as_millis() as u64;

        self.stats.query_count.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_query_latency_ms
            .fetch_add(latency, Ordering::Relaxed);

        match result {
            Ok(points) => {
                self.stats
                    .total_points_read
                    .fetch_add(points.len() as u64, Ordering::Relaxed);
                Ok(points)
            }
            Err(e) => {
                self.stats
                    .total_query_errors
                    .fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    async fn query_raw(&self, query_str: &str) -> Result<Vec<TimeSeriesPoint>> {
        if !self.is_connected().await {
            tracing::warn!("InfluxDB not connected, query_raw returning empty result");
            return Ok(Vec::new());
        }

        let start_time = Instant::now();
        let result = self.do_query_raw(query_str).await;
        let latency = start_time.elapsed().as_millis() as u64;

        self.stats.query_count.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_query_latency_ms
            .fetch_add(latency, Ordering::Relaxed);

        match result {
            Ok(points) => {
                self.stats
                    .total_points_read
                    .fetch_add(points.len() as u64, Ordering::Relaxed);
                Ok(points)
            }
            Err(e) => {
                self.stats
                    .total_query_errors
                    .fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    async fn create_database(&mut self, name: &str) -> Result<()> {
        if !self.is_connected().await {
            tracing::warn!("InfluxDB not connected, skipping bucket creation");
            return Ok(());
        }

        tracing::debug!("Creating bucket: {}", name);

        let bucket_config = serde_json::json!({
            "name": name,
            "orgID": self.org,
            "retentionRules": [{
                "type": "expire",
                "everySeconds": 2592000
            }]
        });

        let result = self
            .retry_with_backoff(|| async {
                let mut request = self
                    .client
                    .post(&self.buckets_url)
                    .json(&bucket_config)
                    .header("Content-Type", "application/json");

                if let Some(token) = &self.config.token {
                    request = request.header("Authorization", format!("Token {}", token));
                }

                let response = request.send().await?;
                let status = response.status();

                if !status.is_success() && status.as_u16() != 422 {
                    let text = response.text().await.unwrap_or_default();
                    return Err(crate::utils::AetherisError::External(format!(
                        "Failed to create bucket: {} - {}",
                        status, text
                    )));
                }

                Ok(())
            })
            .await;

        match result {
            Ok(_) => {
                tracing::info!("Bucket created or already exists: {}", name);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to create bucket {}: {}, continuing", name, e);
                Ok(())
            }
        }
    }

    async fn drop_database(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(vec![self.config.database.clone()])
    }

    async fn create_retention_policy(&mut self, _policy: RetentionPolicy) -> Result<()> {
        Ok(())
    }

    async fn drop_retention_policy(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_retention_policies(&self) -> Result<Vec<RetentionPolicy>> {
        Ok(self.config.retention_policies.clone())
    }

    async fn create_downsampling_rule(&mut self, _rule: DownsamplingRule) -> Result<()> {
        Ok(())
    }

    async fn drop_downsampling_rule(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_downsampling_rules(&self) -> Result<Vec<DownsamplingRule>> {
        Ok(self.config.downsampling_rules.clone())
    }

    async fn ping(&self) -> Result<Duration> {
        let start = Instant::now();

        let result = self
            .retry_with_backoff(|| async {
                let request = self.client.get(&self.health_url);
                let request = if let Some(token) = &self.config.token {
                    request.header("Authorization", format!("Token {}", token))
                } else {
                    request
                };

                let response = request.send().await?;
                if !response.status().is_success() {
                    return Err(crate::utils::AetherisError::External(
                        "Health check failed".to_string(),
                    ));
                }
                Ok(())
            })
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => Ok(elapsed),
            Err(e) => {
                tracing::warn!("InfluxDB ping failed: {}", e);
                Err(e)
            }
        }
    }

    async fn get_stats(&self) -> Result<TimeSeriesStats> {
        Ok(self.stats.to_stats())
    }
}

impl InfluxDBTimeSeries {
    async fn write_lines(&self, lines: Vec<String>) -> Result<()> {
        let num_points = lines.len();
        let start_time = Instant::now();

        if !self.is_connected().await {
            tracing::warn!("InfluxDB not connected, dropping {} points", num_points);
            self.stats
                .total_write_errors
                .fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }

        let body = lines.join("\n");

        let result = self
            .retry_with_backoff(|| async {
                let mut request = self
                    .client
                    .post(&self.write_url)
                    .body(body.clone())
                    .header("Content-Type", "text/plain");

                if let Some(token) = &self.config.token {
                    request = request.header("Authorization", format!("Token {}", token));
                }

                let response = request.send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    return Err(crate::utils::AetherisError::External(format!(
                        "Write failed: {} - {}",
                        status, text
                    )));
                }

                Ok(())
            })
            .await;

        let latency = start_time.elapsed().as_millis() as u64;
        self.stats.write_count.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_write_latency_ms
            .fetch_add(latency, Ordering::Relaxed);

        match result {
            Ok(_) => {
                self.stats
                    .total_points_written
                    .fetch_add(num_points as u64, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                self.stats
                    .total_write_errors
                    .fetch_add(1, Ordering::Relaxed);
                tracing::warn!("Failed to write points: {}", e);
                Ok(())
            }
        }
    }

    async fn do_query(&self, query: TimeSeriesQuery) -> Result<Vec<TimeSeriesPoint>> {
        if !self.is_connected().await {
            tracing::warn!("InfluxDB not connected, query returning empty result");
            return Ok(Vec::new());
        }

        let mut flux_query = String::new();
        flux_query.push_str(&format!(r#"from(bucket: "{}")"#, self.config.database));

        if let Some(start) = query.start_time {
            flux_query.push_str(&format!(r#" |> range(start: {})"#, start.to_rfc3339()));
        } else {
            flux_query.push_str(r#" |> range(start: -1h)"#);
        }

        if let Some(end) = query.end_time {
            flux_query.push_str(&format!(r#" |> range(stop: {})"#, end.to_rfc3339()));
        }

        flux_query.push_str(&format!(
            r#" |> filter(fn: (r) => r._measurement == "{}")"#,
            query.measurement
        ));

        if let Some(tags) = query.tags {
            for (key, values) in tags {
                let values_str = values
                    .iter()
                    .map(|v| format!(r#""{}""#, v))
                    .collect::<Vec<_>>()
                    .join(", ");
                flux_query.push_str(&format!(
                    r#" |> filter(fn: (r) => contains(value: r.{}, set: [{}]))"#,
                    key, values_str
                ));
            }
        }

        if let Some(fields) = query.fields {
            let fields_str = fields
                .iter()
                .map(|f| format!(r#""{}""#, f))
                .collect::<Vec<_>>()
                .join(", ");
            flux_query.push_str(&format!(
                r#" |> filter(fn: (r) => contains(value: r._field, set: [{}]))"#,
                fields_str
            ));
        }

        if let Some(order) = query.order {
            match order {
                QueryOrder::Ascending => flux_query.push_str(r#" |> sort(columns: ["_time"])"#),
                QueryOrder::Descending => {
                    flux_query.push_str(r#" |> sort(columns: ["_time"], desc: true)"#)
                }
            }
        }

        if let Some(limit) = query.limit {
            flux_query.push_str(&format!(r#" |> limit(n: {})"#, limit));
        }

        self.do_query_raw(&flux_query).await
    }

    async fn do_query_raw(&self, query_str: &str) -> Result<Vec<TimeSeriesPoint>> {
        if !self.is_connected().await {
            return Ok(Vec::new());
        }

        tracing::debug!("Executing Flux query: {}", query_str);

        let query_data = serde_json::json!({
            "query": query_str,
            "type": "flux"
        });

        let result = self
            .retry_with_backoff(|| async {
                let mut request = self
                    .client
                    .post(&self.query_url)
                    .json(&query_data)
                    .header("Content-Type", "application/json");

                if let Some(token) = &self.config.token {
                    request = request.header("Authorization", format!("Token {}", token));
                }

                let response = request.send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    return Err(crate::utils::AetherisError::External(format!(
                        "Query failed: {} - {}",
                        status, text
                    )));
                }

                let text = response.text().await?;
                Ok(text)
            })
            .await;

        match result {
            Ok(text) => self.parse_flux_response(&text),
            Err(e) => {
                tracing::warn!("Query execution failed: {}", e);
                Ok(Vec::new())
            }
        }
    }
}

fn escape_tag_key(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

fn escape_tag_value(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

fn escape_field_key(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

fn format_field_value(value: &TimeSeriesValue) -> String {
    match value {
        TimeSeriesValue::Boolean(b) => (if *b { "true" } else { "false" }).to_string(),
        TimeSeriesValue::Int64(i) => format!("{}i", i),
        TimeSeriesValue::UInt64(u) => format!("{}u", u),
        TimeSeriesValue::Float64(f) => format!("{}", f),
        TimeSeriesValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
    }
}

pub struct InfluxDBFactory;

impl TimeSeriesDatabaseFactory for InfluxDBFactory {
    fn create(&self, config: TimeSeriesConfig) -> Arc<RwLock<dyn TimeSeriesDatabase + Send + Sync>> {
        Arc::new(RwLock::new(InfluxDBTimeSeries::new(config)))
    }

    fn supported_backends(&self) -> Vec<TimeSeriesBackendType> {
        vec![TimeSeriesBackendType::InfluxDB]
    }
}
