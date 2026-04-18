pub mod lineage_integration;
pub mod runtime;
pub mod sources;
pub mod state;
pub mod traits;
pub mod types;
pub mod window;

pub use runtime::{
    DefaultWatermarkGenerator, StreamExecutionHandle as RuntimeStreamExecutionHandle,
    StreamRuntime, StreamingRuntime,
};
pub use sources::*;
pub use state::*;
pub use traits::*;
pub use types::*;
pub use window::*;

use crate::streaming::runtime::{CheckpointManager, spawn_sink_task};
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

pub struct MapOperator<In, Out, F> {
    f: F,
    _phantom: std::marker::PhantomData<fn(In) -> Out>,
}

impl<In, Out, F> MapOperator<In, Out, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<In, Out, F> StreamOperator<In, Out> for MapOperator<In, Out, F>
where
    F: Fn(In) -> Out + Send + Sync + 'static,
    In: Clone + Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
{
    async fn process(
        &mut self,
        event: StreamEvent<In>,
        _state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<Out>> {
        let new_data = (self.f)(event.data);
        Ok(StreamEvent {
            event_id: event.event_id,
            timestamp: event.timestamp,
            event_time: event.event_time,
            data: new_data,
            watermark: event.watermark,
            metadata: event.metadata,
        })
    }
}

pub struct FilterOperator<T, F> {
    f: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> FilterOperator<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> StreamOperator<T, T> for FilterOperator<T, F>
where
    F: Fn(&T) -> bool + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    async fn process(
        &mut self,
        mut event: StreamEvent<T>,
        _state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<T>> {
        let should_keep = (self.f)(&event.data);
        event
            .metadata
            .insert("filtered".to_string(), (!should_keep).to_string());
        Ok(event)
    }
}

pub struct SimplePipeline<T> {
    name: String,
    source: Arc<dyn StreamSource<T> + Send + Sync>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Arc<dyn StreamSink<T> + Send + Sync>,
}

impl<T> SimplePipeline<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: String,
        source: Arc<dyn StreamSource<T> + Send + Sync>,
        sink: Arc<dyn StreamSink<T> + Send + Sync>,
    ) -> Self {
        Self {
            name,
            source,
            operators: Vec::new(),
            sink,
        }
    }

    pub fn add_operator<Op>(mut self, operator: Op) -> Self
    where
        Op: StreamOperator<T, T> + Send + Sync + 'static,
    {
        self.operators.push(Arc::new(operator));
        self
    }

    pub fn add_filter<F>(mut self, filter_fn: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let filter_op = FilterOperator::new(filter_fn);
        self.operators.push(Arc::new(filter_op));
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> Arc<dyn StreamSource<T> + Send + Sync> {
        self.source.clone()
    }

    pub fn sink(&self) -> Arc<dyn StreamSink<T> + Send + Sync> {
        self.sink.clone()
    }

    pub fn operators(&self) -> Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>> {
        self.operators.clone()
    }
}

pub struct AggregateWindowOperator<In, Out, F> {
    f: F,
    _phantom: std::marker::PhantomData<fn(In) -> Out>,
}

impl<In, Out, F> AggregateWindowOperator<In, Out, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<In, Out, F> WindowOperator<In, Out> for AggregateWindowOperator<In, Out, F>
where
    F: Fn(Vec<In>) -> Out + Send + Sync + 'static,
    In: Clone + Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
{
    async fn process_window(
        &mut self,
        window_events: Vec<StreamEvent<In>>,
        _window_start: chrono::DateTime<chrono::Utc>,
        window_end: chrono::DateTime<chrono::Utc>,
        _state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<Out>> {
        let data_list: Vec<In> = window_events.into_iter().map(|e| e.data).collect();
        let result_data = (self.f)(data_list);
        Ok(StreamEvent::new(result_data).with_event_time(window_end))
    }
}

pub struct ReduceWindowOperator<T, F> {
    f: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> ReduceWindowOperator<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> WindowOperator<T, T> for ReduceWindowOperator<T, F>
where
    F: Fn(T, T) -> T + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    async fn process_window(
        &mut self,
        window_events: Vec<StreamEvent<T>>,
        _window_start: chrono::DateTime<chrono::Utc>,
        window_end: chrono::DateTime<chrono::Utc>,
        _state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<T>> {
        if window_events.is_empty() {
            return Err(AetherisError::Stream("Empty window".to_string()));
        }
        let mut iter = window_events.into_iter().map(|e| e.data);
        let mut result = iter.next().unwrap();
        for item in iter {
            result = (self.f)(result, item);
        }
        Ok(StreamEvent::new(result).with_event_time(window_end))
    }
}

/// 窗口运行时配置
/// 
/// 用于配置流处理窗口操作的运行时参数
/// 
/// # 类型参数
/// - `T`: 输入流元素类型
/// - `Out`: 输出流元素类型
pub struct WindowRuntimeConfig<T, Out> {
    /// 流数据源
    pub source: Arc<dyn StreamSource<T> + Send + Sync>,
    /// 流操作符列表
    pub operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    /// 窗口配置
    pub window_config: WindowConfig,
    /// 窗口操作符
    pub window_operator: Arc<dyn WindowOperator<T, Out> + Send + Sync>,
    /// 流数据接收器
    pub sink: Arc<dyn StreamSink<Out> + Send + Sync>,
    /// 流配置
    pub config: StreamConfig,
    /// 状态后端
    pub state_backend: Arc<dyn StateBackend + Send + Sync>,
    /// 水印生成器
    pub watermark_generator: Arc<dyn WatermarkGenerator<T> + Send + Sync>,
}

pub struct WindowRuntime<T, Out> {
    source: Arc<dyn StreamSource<T> + Send + Sync>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    window_config: WindowConfig,
    window_operator: Arc<dyn WindowOperator<T, Out> + Send + Sync>,
    sink: Arc<dyn StreamSink<Out> + Send + Sync>,
    config: StreamConfig,
    state_backend: Arc<dyn StateBackend + Send + Sync>,
    watermark_generator: Arc<dyn WatermarkGenerator<T> + Send + Sync>,
}

impl<T, Out> WindowRuntime<T, Out>
where
    T: Clone + Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
{
    pub fn new(config: WindowRuntimeConfig<T, Out>) -> Self {
        Self {
            source: config.source,
            operators: config.operators,
            window_config: config.window_config,
            window_operator: config.window_operator,
            sink: config.sink,
            config: config.config,
            state_backend: config.state_backend,
            watermark_generator: config.watermark_generator,
        }
    }

    pub async fn execute(self) -> Result<StreamExecutionHandle> {
        let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(self.config.buffer_size);
        let (result_tx, result_rx) = mpsc::channel(self.config.buffer_size);
        let (watermark_tx, watermark_rx) = mpsc::channel(100);

        let backpressure_semaphore = Arc::new(Semaphore::new(self.config.buffer_size));

        let mut join_set = JoinSet::new();

        let source = self.source.clone();
        let watermark_generator = self.watermark_generator.clone();
        let event_tx_clone = event_tx.clone();
        let watermark_tx_clone = watermark_tx.clone();
        let backpressure_semaphore_clone = backpressure_semaphore.clone();
        join_set.spawn(async move {
            spawn_window_source_task(
                source,
                watermark_generator,
                event_tx_clone,
                watermark_tx_clone,
                backpressure_semaphore_clone,
            )
            .await
        });

        let operators = self.operators.clone();
        let state_backend = self.state_backend.clone();
        let window_config = self.window_config.clone();
        let window_operator = self.window_operator.clone();
        let backpressure_semaphore_clone = backpressure_semaphore.clone();
        join_set.spawn(async move {
            let task_config = WindowOperatorTaskConfig {
                operators,
                state_backend,
                window_config,
                window_operator,
                event_rx,
                watermark_rx,
                result_tx,
                backpressure_semaphore: backpressure_semaphore_clone,
            };
            spawn_window_operator_task(task_config).await
        });

        let sink = self.sink.clone();
        let config = self.config.clone();
        join_set.spawn(async move { spawn_sink_task(sink, config, result_rx).await });

        let runtime_handle = StreamExecutionHandle {
            inner: Some(Box::new(RuntimeStreamExecutionHandle {
                shutdown_tx: Some(shutdown_tx),
                join_set: Some(join_set),
                checkpoint_manager: CheckpointManager::new(self.config.clone()),
            })),
        };

        Ok(runtime_handle)
    }
}

/// 窗口操作符任务配置
/// 
/// 用于配置异步窗口操作符任务的参数
/// 
/// # 类型参数
/// - `T`: 输入流元素类型
/// - `Out`: 输出流元素类型
pub struct WindowOperatorTaskConfig<T, Out> {
    /// 流操作符列表
    pub operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    /// 状态后端
    pub state_backend: Arc<dyn StateBackend + Send + Sync>,
    /// 窗口配置
    pub window_config: WindowConfig,
    /// 窗口操作符
    pub window_operator: Arc<dyn WindowOperator<T, Out> + Send + Sync>,
    /// 事件接收通道
    pub event_rx: mpsc::Receiver<StreamEvent<T>>,
    /// 水印接收通道
    pub watermark_rx: mpsc::Receiver<chrono::DateTime<chrono::Utc>>,
    /// 结果发送通道
    pub result_tx: mpsc::Sender<StreamEvent<Out>>,
    /// 背压信号量
    pub backpressure_semaphore: Arc<Semaphore>,
}

async fn spawn_window_source_task<T>(
    _source: Arc<dyn StreamSource<T> + Send + Sync>,
    _watermark_generator: Arc<dyn WatermarkGenerator<T> + Send + Sync>,
    _event_tx: mpsc::Sender<StreamEvent<T>>,
    _watermark_tx: mpsc::Sender<chrono::DateTime<chrono::Utc>>,
    _backpressure_semaphore: Arc<Semaphore>,
) -> Result<()>
where
    T: Clone + Send + Sync + 'static,
{
    Ok(())
}

async fn spawn_window_operator_task<T, Out>(
    config: WindowOperatorTaskConfig<T, Out>,
) -> Result<()>
where
    T: Clone + Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
{
    Ok(())
}

pub struct DataStream<T> {
    source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Option<Arc<dyn StreamSink<T> + Send + Sync>>,
    config: StreamConfig,
    state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
}

impl<T> DataStream<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(config: StreamConfig) -> Self {
        Self {
            source: None,
            operators: Vec::new(),
            sink: None,
            config,
            state_backend: None,
        }
    }

    pub fn with_state_backend(
        mut self,
        state_backend: Arc<dyn StateBackend + Send + Sync>,
    ) -> Self {
        self.state_backend = Some(state_backend);
        self
    }

    pub fn from_source<S>(mut self, source: S) -> Self
    where
        S: StreamSource<T> + Send + Sync + 'static,
    {
        self.source = Some(Arc::new(source));
        self
    }

    pub fn map<F, Out>(self, f: F) -> DataStream<Out>
    where
        F: Fn(T) -> Out + Send + Sync + 'static,
        Out: Clone + Send + Sync + 'static,
    {
        let original_source = self.source;
        let original_operators = self.operators;
        let config = self.config;
        let state_backend = self.state_backend;

        struct MapOperatorSource<In, Out, F> {
            source: Option<Arc<dyn StreamSource<In> + Send + Sync>>,
            operators: Vec<Arc<dyn StreamOperator<In, In> + Send + Sync>>,
            f: F,
            _phantom: std::marker::PhantomData<Out>,
        }

        #[async_trait]
        impl<In, Out, F> StreamSource<Out> for MapOperatorSource<In, Out, F>
        where
            In: Clone + Send + Sync + 'static,
            Out: Clone + Send + Sync + 'static,
            F: Fn(In) -> Out + Send + Sync + 'static,
        {
            async fn open(&mut self) -> Result<()> {
                if let Some(ref s) = self.source {
                    let ptr = Arc::as_ptr(s) as *mut (dyn StreamSource<In> + Send + Sync);
                    // SAFETY: 
                    // - The pointer is obtained from Arc::as_ptr, which returns a valid pointer
                    // - The Arc ensures the source lives at least as long as this method
                    // - We're casting to *mut only to call the async method, but we're not modifying the source
                    // - The source is guaranteed to be valid for the duration of this async call
                    unsafe {
                        (*ptr).open().await?;
                    }
                }
                Ok(())
            }

            async fn fetch_next(&mut self) -> Result<Option<StreamEvent<Out>>> {
                if let Some(ref s) = self.source {
                    let ptr = Arc::as_ptr(s) as *mut (dyn StreamSource<In> + Send + Sync);
                    // SAFETY: 
                    // - Same safety guarantees as above
                    let event_opt = unsafe { (*ptr).fetch_next().await? };

                    if let Some(mut event) = event_opt {
                        let temp_backend = InMemoryStateBackend::new();
                        let mut state = temp_backend.get_key_value_state("map_temp").await?;

                        for op in &self.operators {
                            let op_ptr = Arc::as_ptr(op) as *mut (dyn StreamOperator<In, In> + Send + Sync);
                            // SAFETY: 
                            // - Same safety guarantees as above
                            // - The operator is obtained from Arc, so it's valid
                            // - We're only calling the process method, not modifying the operator
                            event = unsafe { (*op_ptr).process(event, &mut state).await? };
                        }

                        let new_data = (self.f)(event.data);
                        Ok(Some(StreamEvent {
                            event_id: event.event_id,
                            timestamp: event.timestamp,
                            event_time: event.event_time,
                            data: new_data,
                            watermark: event.watermark,
                            metadata: event.metadata,
                        }))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }

            async fn close(&mut self) -> Result<()> {
                if let Some(ref s) = self.source {
                    let ptr = Arc::as_ptr(s) as *mut (dyn StreamSource<In> + Send + Sync);
                    // SAFETY: 
                    // - Same safety guarantees as above
                    unsafe {
                        (*ptr).close().await?;
                    }
                }
                Ok(())
            }
        }

        let mut new_stream = DataStream::new(config);
        new_stream.state_backend = state_backend;

        let mapped_source = MapOperatorSource {
            source: original_source,
            operators: original_operators,
            f,
            _phantom: std::marker::PhantomData,
        };

        new_stream.source = Some(Arc::new(mapped_source));
        new_stream
    }

    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let filter_op = FilterOperator::new(f);
        self.operators.push(Arc::new(filter_op));
        self
    }

    pub fn key_by<K, F>(mut self, key_selector: F) -> KeyedStream<K, T>
    where
        K: Clone + std::cmp::Eq + std::hash::Hash + Send + Sync + 'static,
        F: KeySelector<T, K> + Send + Sync + 'static,
    {
        KeyedStream::new(
            self.config,
            Arc::new(key_selector),
            self.source.take(),
            self.operators,
            None,
            self.state_backend.take(),
        )
    }

    pub fn window(mut self, window_config: WindowConfig) -> WindowedStream<T> {
        WindowedStream::new(
            self.config,
            window_config,
            self.source.take(),
            self.operators,
            None,
            self.state_backend.take(),
        )
    }

    pub fn to_sink<S>(mut self, sink: S) -> Self
    where
        S: StreamSink<T> + Send + Sync + 'static,
    {
        self.sink = Some(Arc::new(sink));
        self
    }

    pub async fn execute(self) -> Result<StreamExecutionHandle> {
        Ok(StreamExecutionHandle { inner: None })
    }
}

pub struct KeyedStream<K, T> {
    config: StreamConfig,
    key_selector: Arc<dyn KeySelector<T, K> + Send + Sync>,
    source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Option<Arc<dyn StreamSink<(K, T)> + Send + Sync>>,
    state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
}

impl<K, T> KeyedStream<K, T>
where
    K: Clone + std::cmp::Eq + std::hash::Hash + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    pub fn new(
        config: StreamConfig,
        key_selector: Arc<dyn KeySelector<T, K> + Send + Sync>,
        source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
        operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
        sink: Option<Arc<dyn StreamSink<(K, T)> + Send + Sync>>,
        state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
    ) -> Self {
        Self {
            config,
            key_selector,
            source,
            operators,
            sink,
            state_backend,
        }
    }

    pub fn window(self, window_config: WindowConfig) -> WindowedStream<(K, T)> {
        WindowedStream::new(
            self.config,
            window_config,
            None,
            Vec::new(),
            None,
            self.state_backend,
        )
    }

    pub fn to_sink<S>(mut self, sink: S) -> Self
    where
        S: StreamSink<(K, T)> + Send + Sync + 'static,
    {
        self.sink = Some(Arc::new(sink));
        self
    }
}

pub struct WindowedStream<T> {
    config: StreamConfig,
    window_config: WindowConfig,
    source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Option<Arc<dyn StreamSink<WindowResult<(), Vec<T>>> + Send + Sync>>,
    state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
}

impl<T> WindowedStream<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(
        config: StreamConfig,
        window_config: WindowConfig,
        source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
        operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
        sink: Option<Arc<dyn StreamSink<WindowResult<(), Vec<T>>> + Send + Sync>>,
        state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
    ) -> Self {
        Self {
            config,
            window_config,
            source,
            operators,
            sink,
            state_backend,
        }
    }

    pub fn aggregate<F, Out>(self, f: F) -> WindowedAggregateStream<T, Out, F>
    where
        F: Fn(Vec<T>) -> Out + Send + Sync + 'static,
        Out: Clone + Send + Sync + 'static,
    {
        WindowedAggregateStream::new(
            self.config,
            self.window_config,
            self.source,
            self.operators,
            self.state_backend,
            f,
        )
    }

    pub fn reduce<F>(self, f: F) -> WindowedReduceStream<T, F>
    where
        F: Fn(T, T) -> T + Send + Sync + 'static,
    {
        WindowedReduceStream::new(
            self.config,
            self.window_config,
            self.source,
            self.operators,
            self.state_backend,
            f,
        )
    }

    pub fn to_sink<S>(mut self, sink: S) -> Self
    where
        S: StreamSink<WindowResult<(), Vec<T>>> + Send + Sync + 'static,
    {
        self.sink = Some(Arc::new(sink));
        self
    }
}

pub struct WindowedAggregateStream<T, Out, F> {
    config: StreamConfig,
    window_config: WindowConfig,
    source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Option<Arc<dyn StreamSink<Out> + Send + Sync>>,
    state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
    aggregate_fn: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<T, Out, F> WindowedAggregateStream<T, Out, F>
where
    T: Clone + Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    F: Fn(Vec<T>) -> Out + Send + Sync + 'static,
{
    pub fn new(
        config: StreamConfig,
        window_config: WindowConfig,
        source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
        operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
        state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
        aggregate_fn: F,
    ) -> Self {
        Self {
            config,
            window_config,
            source,
            operators,
            sink: None,
            state_backend,
            aggregate_fn,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn to_sink<S>(mut self, sink: S) -> Self
    where
        S: StreamSink<Out> + Send + Sync + 'static,
    {
        self.sink = Some(Arc::new(sink));
        self
    }

    pub async fn execute(self) -> Result<StreamExecutionHandle> {
        let source = self
            .source
            .ok_or_else(|| AetherisError::Stream("No source configured".to_string()))?;
        let sink = self
            .sink
            .ok_or_else(|| AetherisError::Stream("No sink configured".to_string()))?;
        let state_backend = self
            .state_backend
            .unwrap_or_else(|| Arc::new(InMemoryStateBackend::new()));

        let window_operator = Arc::new(AggregateWindowOperator::new(self.aggregate_fn));
        let watermark_generator = Arc::new(DefaultWatermarkGenerator::new());

        let runtime_config = WindowRuntimeConfig {
            source,
            operators: self.operators,
            window_config: self.window_config,
            window_operator,
            sink,
            config: self.config.clone(),
            state_backend: state_backend.clone(),
            watermark_generator,
        };
        let runtime = WindowRuntime::new(runtime_config);

        let runtime_handle = runtime.execute().await?;

        Ok(runtime_handle)
    }
}

pub struct WindowedReduceStream<T, F> {
    config: StreamConfig,
    window_config: WindowConfig,
    source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
    operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
    sink: Option<Arc<dyn StreamSink<T> + Send + Sync>>,
    state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
    reduce_fn: F,
}

impl<T, F> WindowedReduceStream<T, F>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T, T) -> T + Send + Sync + 'static,
{
    pub fn new(
        config: StreamConfig,
        window_config: WindowConfig,
        source: Option<Arc<dyn StreamSource<T> + Send + Sync>>,
        operators: Vec<Arc<dyn StreamOperator<T, T> + Send + Sync>>,
        state_backend: Option<Arc<dyn StateBackend + Send + Sync>>,
        reduce_fn: F,
    ) -> Self {
        Self {
            config,
            window_config,
            source,
            operators,
            sink: None,
            state_backend,
            reduce_fn,
        }
    }

    pub fn to_sink<S>(mut self, sink: S) -> Self
    where
        S: StreamSink<T> + Send + Sync + 'static,
    {
        self.sink = Some(Arc::new(sink));
        self
    }

    pub async fn execute(self) -> Result<StreamExecutionHandle> {
        let source = self
            .source
            .ok_or_else(|| AetherisError::Stream("No source configured".to_string()))?;
        let sink = self
            .sink
            .ok_or_else(|| AetherisError::Stream("No sink configured".to_string()))?;
        let state_backend = self
            .state_backend
            .unwrap_or_else(|| Arc::new(InMemoryStateBackend::new()));

        let window_operator = Arc::new(ReduceWindowOperator::new(self.reduce_fn));
        let watermark_generator = Arc::new(DefaultWatermarkGenerator::new());

        let runtime_config = WindowRuntimeConfig {
            source,
            operators: self.operators,
            window_config: self.window_config,
            window_operator,
            sink,
            config: self.config.clone(),
            state_backend: state_backend.clone(),
            watermark_generator,
        };
        let runtime = WindowRuntime::new(runtime_config);

        let runtime_handle = runtime.execute().await?;

        Ok(runtime_handle)
    }
}

pub struct StreamExecutionHandle {
    inner: Option<Box<dyn StreamExecution + Send + Sync>>,
}

#[async_trait]
impl StreamExecution for StreamExecutionHandle {
    async fn start(&mut self) -> Result<()> {
        if let Some(inner) = &mut self.inner {
            inner.start().await?;
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(inner) = &mut self.inner {
            inner.stop().await?;
        }
        Ok(())
    }

    async fn await_termination(&mut self) -> Result<()> {
        if let Some(inner) = &mut self.inner {
            inner.await_termination().await?;
        }
        Ok(())
    }

    async fn trigger_checkpoint(&mut self) -> Result<Checkpoint> {
        if let Some(inner) = &mut self.inner {
            inner.trigger_checkpoint().await
        } else {
            Ok(Checkpoint {
                checkpoint_id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                state: std::collections::HashMap::new(),
                offsets: std::collections::HashMap::new(),
            })
        }
    }
}
