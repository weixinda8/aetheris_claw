use crate::streaming::traits::*;
use crate::streaming::types::*;
use crate::utils::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

type WindowKey<K> = (K, chrono::DateTime<chrono::Utc>);
type WindowMap<K, V> = HashMap<WindowKey<K>, Vec<StreamEvent<V>>>;
type WindowStorage<K, V> = Arc<RwLock<WindowMap<K, V>>>;

pub struct WindowAssigner<K, V> {
    config: WindowConfig,
    windows: WindowStorage<K, V>,
}

impl<K, V> WindowAssigner<K, V>
where
    K: Clone + std::cmp::Eq + std::hash::Hash + Send + Sync,
    V: Clone + Send + Sync,
{
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn assign(
        &self,
        key: K,
        event: StreamEvent<V>,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>> {
        let mut windows = Vec::new();
        let event_time = event.event_time;

        match self.config.window_type {
            WindowType::Tumbling => {
                let window_start = self.truncate_to_window(event_time);
                let window_end = window_start + self.config.size;
                windows.push((window_start, window_end));

                let mut state = self.windows.write().await;
                state
                    .entry((key, window_start))
                    .or_default()
                    .push(event);
            }
            WindowType::Sliding => {
                let slide = self.config.slide.unwrap_or(self.config.size);
                let start_time = event_time - self.config.size + slide;
                let mut current = self.truncate_to_window(start_time);

                while current <= event_time {
                    let window_start = current;
                    let window_end = window_start + self.config.size;
                    windows.push((window_start, window_end));

                    let mut state = self.windows.write().await;
                    state
                        .entry((key.clone(), window_start))
                        .or_default()
                        .push(event.clone());

                    current += slide;
                }
            }
            WindowType::Session => {
                let gap = self.config.gap.unwrap_or(Duration::from_secs(300));
                let gap_td = chrono::Duration::milliseconds(gap.as_millis() as i64);
                let mut state = self.windows.write().await;

                let mut found_existing = false;
                for ((k, start), events) in state.iter_mut() {
                    if k == &key {
                        if let Some(last_event) = events.last() {
                            if event_time - last_event.event_time < gap_td {
                                events.push(event.clone());
                                windows.push((*start, event_time + gap_td));
                                found_existing = true;
                                break;
                            }
                        }
                    }
                }

                if !found_existing {
                    let window_start = event_time;
                    let window_end = window_start + gap_td;
                    windows.push((window_start, window_end));
                    state.insert((key, window_start), vec![event]);
                }
            }
        }

        Ok(windows)
    }

    pub async fn trigger_windows(
        &self,
        watermark: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<WindowResult<K, Vec<StreamEvent<V>>>>> {
        let mut state = self.windows.write().await;
        let mut results = Vec::new();
        let mut to_remove = Vec::new();

        match self.config.window_type {
            WindowType::Tumbling | WindowType::Sliding => {
                for ((key, window_start), events) in state.iter() {
                    let window_end = *window_start + self.config.size;
                    if window_end + self.config.allowed_lateness <= watermark {
                        results.push(WindowResult {
                            window_start: *window_start,
                            window_end,
                            key: key.clone(),
                            value: events.clone(),
                        });
                        to_remove.push((key.clone(), *window_start));
                    }
                }
            }
            WindowType::Session => {
                let gap = self.config.gap.unwrap_or(Duration::from_secs(300));
                for ((key, window_start), events) in state.iter() {
                    if let Some(last_event) = events.last() {
                        let session_end = last_event.event_time + gap;
                        if session_end + self.config.allowed_lateness <= watermark {
                            results.push(WindowResult {
                                window_start: *window_start,
                                window_end: session_end,
                                key: key.clone(),
                                value: events.clone(),
                            });
                            to_remove.push((key.clone(), *window_start));
                        }
                    }
                }
            }
        }

        for (key, window_start) in to_remove {
            state.remove(&(key, window_start));
        }

        Ok(results)
    }

    fn truncate_to_window(
        &self,
        time: chrono::DateTime<chrono::Utc>,
    ) -> chrono::DateTime<chrono::Utc> {
        let nanos = time.timestamp_nanos_opt().unwrap();
        let window_nanos = self.config.size.as_nanos() as i64;
        let truncated = nanos - (nanos % window_nanos);
        chrono::DateTime::from_timestamp_nanos(truncated)
    }
}

pub struct TumblingWatermarkGenerator<T> {
    current_watermark: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    delay: Duration,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TumblingWatermarkGenerator<T> {
    pub fn new(delay: Duration) -> Self {
        Self {
            current_watermark: Arc::new(RwLock::new(None)),
            delay,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Send + Sync> WatermarkGenerator<T> for TumblingWatermarkGenerator<T> {
    fn on_event(&mut self, _event: &StreamEvent<T>) {
    }

    fn get_watermark(&self, event: &StreamEvent<T>) -> Option<chrono::DateTime<chrono::Utc>> {
        let new_watermark = event.event_time - self.delay;
        Some(new_watermark)
    }

    fn get_current_watermark(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.current_watermark.blocking_read()
    }
}

pub struct BoundedOutOfOrdernessWatermarkGenerator<T> {
    current_watermark: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    max_out_of_orderness: Duration,
    latest_timestamp: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> BoundedOutOfOrdernessWatermarkGenerator<T> {
    pub fn new(max_out_of_orderness: Duration) -> Self {
        Self {
            current_watermark: Arc::new(RwLock::new(None)),
            max_out_of_orderness,
            latest_timestamp: Arc::new(RwLock::new(None)),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Send + Sync> WatermarkGenerator<T> for BoundedOutOfOrdernessWatermarkGenerator<T> {
    fn on_event(&mut self, _event: &StreamEvent<T>) {
    }

    fn get_watermark(&self, event: &StreamEvent<T>) -> Option<chrono::DateTime<chrono::Utc>> {
        let mut latest = self.latest_timestamp.blocking_write();
        if latest.is_none() || event.event_time > latest.unwrap() {
            *latest = Some(event.event_time);
        }

        let new_watermark = latest.unwrap() - self.max_out_of_orderness;
        let mut current = self.current_watermark.blocking_write();
        if current.is_none() || new_watermark > current.unwrap() {
            *current = Some(new_watermark);
        }

        *current
    }

    fn get_current_watermark(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.current_watermark.blocking_read()
    }
}
