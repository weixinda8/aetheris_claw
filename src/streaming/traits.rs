use crate::streaming::state::KeyValueState;
use crate::streaming::types::*;
use crate::utils::Result;
use async_trait::async_trait;

#[async_trait]
pub trait StreamSource<T>: Send + Sync {
    async fn open(&mut self) -> Result<()>;
    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<T>>>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait StreamSink<T>: Send + Sync {
    async fn open(&mut self) -> Result<()>;
    async fn write(&mut self, event: StreamEvent<T>) -> Result<()>;
    async fn write_batch(&mut self, events: Vec<StreamEvent<T>>) -> Result<()>;
    async fn flush(&mut self) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait StreamOperator<In, Out>: Send + Sync {
    async fn process(
        &mut self,
        event: StreamEvent<In>,
        state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<Out>>;
}

pub trait StreamFunction<In, Out>: Send + Sync {
    fn apply(&self, input: In) -> Out;
}

pub trait KeySelector<T, K>: Send + Sync {
    fn select_key(&self, value: &T) -> K;
}

#[async_trait]
pub trait StateBackend: Send + Sync {
    async fn get(&self, key: String) -> Result<Option<Vec<u8>>>;

    async fn put(&mut self, key: String, value: Vec<u8>) -> Result<()>;

    async fn delete(&mut self, key: String) -> Result<()>;

    async fn save_checkpoint(&self) -> Result<Checkpoint>;
    async fn load_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()>;

    async fn get_key_value_state(&self, name: &str) -> Result<KeyValueState<String, String>>;
}

pub trait WatermarkGenerator<T>: Send + Sync {
    fn on_event(&mut self, event: &StreamEvent<T>);
    fn get_watermark(&self, event: &StreamEvent<T>) -> Option<chrono::DateTime<chrono::Utc>>;
    fn get_current_watermark(&self) -> Option<chrono::DateTime<chrono::Utc>>;
}

#[async_trait]
pub trait WindowOperator<In, Out>: Send + Sync {
    async fn process_window(
        &mut self,
        window_events: Vec<StreamEvent<In>>,
        window_start: chrono::DateTime<chrono::Utc>,
        window_end: chrono::DateTime<chrono::Utc>,
        state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<Out>>;
}

#[async_trait]
pub trait StreamExecution: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn await_termination(&mut self) -> Result<()>;
    async fn trigger_checkpoint(&mut self) -> Result<Checkpoint>;
}
