use crate::skill::{CallMode, Skill, SkillMetadata};
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCallRequest {
    pub call_mode: CallMode,
    pub payload: Value,
    pub options: Option<CallOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallOptions {
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub headers: Option<HashMap<String, String>>,
    pub metadata: Option<HashMap<String, String>>,
}

impl Default for CallOptions {
    fn default() -> Self {
        Self {
            timeout_ms: Some(30000),
            retries: Some(3),
            headers: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCallResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
    pub execution_time_ms: u64,
}

#[async_trait]
pub trait UnifiedCallHandler: Send + Sync {
    fn call_mode(&self) -> CallMode;
    async fn execute(&self, request: UnifiedCallRequest) -> Result<UnifiedCallResponse>;
    fn can_handle(&self, mode: &CallMode) -> bool {
        self.call_mode() == *mode || *mode == CallMode::Hybrid
    }
}

pub struct TextHandler;

impl Default for TextHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TextHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UnifiedCallHandler for TextHandler {
    fn call_mode(&self) -> CallMode {
        CallMode::Text
    }

    async fn execute(&self, request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        let start = std::time::Instant::now();
        info!("Executing text call");

        let result = match request.payload {
            Value::String(text) => Ok(Value::String(format!("Processed: {}", text))),
            _ => Err(AetherisError::Skill(
                "Text payload must be a string".to_string(),
            )),
        };

        let duration = start.elapsed();
        let execution_time_ms = duration.as_millis() as u64;

        match result {
            Ok(data) => Ok(UnifiedCallResponse {
                success: true,
                data: Some(data),
                error: None,
                metadata: HashMap::new(),
                execution_time_ms,
            }),
            Err(e) => Ok(UnifiedCallResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
                metadata: HashMap::new(),
                execution_time_ms,
            }),
        }
    }
}

pub struct DatabaseHandler;

impl Default for DatabaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UnifiedCallHandler for DatabaseHandler {
    fn call_mode(&self) -> CallMode {
        CallMode::Database
    }

    async fn execute(&self, _request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        let start = std::time::Instant::now();
        info!("Executing database call");

        let duration = start.elapsed();
        let execution_time_ms = duration.as_millis() as u64;

        Ok(UnifiedCallResponse {
            success: true,
            data: Some(Value::Array(Vec::new())),
            error: None,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("rows_affected".to_string(), "0".to_string());
                meta
            },
            execution_time_ms,
        })
    }
}

pub struct ImageHandler;

impl Default for ImageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UnifiedCallHandler for ImageHandler {
    fn call_mode(&self) -> CallMode {
        CallMode::Image
    }

    async fn execute(&self, _request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        let start = std::time::Instant::now();
        info!("Executing image call");

        let duration = start.elapsed();
        let execution_time_ms = duration.as_millis() as u64;

        Ok(UnifiedCallResponse {
            success: true,
            data: Some(Value::Object(serde_json::Map::new())),
            error: None,
            metadata: HashMap::new(),
            execution_time_ms,
        })
    }
}

pub struct AudioHandler;

impl Default for AudioHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UnifiedCallHandler for AudioHandler {
    fn call_mode(&self) -> CallMode {
        CallMode::Audio
    }

    async fn execute(&self, _request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        let start = std::time::Instant::now();
        info!("Executing audio call");

        let duration = start.elapsed();
        let execution_time_ms = duration.as_millis() as u64;

        Ok(UnifiedCallResponse {
            success: true,
            data: Some(Value::Object(serde_json::Map::new())),
            error: None,
            metadata: HashMap::new(),
            execution_time_ms,
        })
    }
}

pub struct HybridHandler {
    handlers: Vec<Box<dyn UnifiedCallHandler>>,
}

impl Default for HybridHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridHandler {
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn UnifiedCallHandler>> = vec![
            Box::new(TextHandler::new()),
            Box::new(DatabaseHandler::new()),
            Box::new(ImageHandler::new()),
            Box::new(AudioHandler::new()),
        ];

        Self { handlers }
    }

    pub fn with_handler(mut self, handler: Box<dyn UnifiedCallHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
}

#[async_trait]
impl UnifiedCallHandler for HybridHandler {
    fn call_mode(&self) -> CallMode {
        CallMode::Hybrid
    }

    async fn execute(&self, request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        let start = std::time::Instant::now();
        info!("Executing hybrid call for mode: {:?}", request.call_mode);

        for handler in &self.handlers {
            if handler.can_handle(&request.call_mode) && handler.call_mode() == request.call_mode {
                let result = handler.execute(request).await;
                let duration = start.elapsed();
                let execution_time_ms = duration.as_millis() as u64;

                return match result {
                    Ok(mut response) => {
                        response.execution_time_ms = execution_time_ms;
                        Ok(response)
                    }
                    Err(e) => Ok(UnifiedCallResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                        metadata: HashMap::new(),
                        execution_time_ms,
                    }),
                };
            }
        }

        let duration = start.elapsed();
        Ok(UnifiedCallResponse {
            success: false,
            data: None,
            error: Some(format!("No handler for mode: {:?}", request.call_mode)),
            metadata: HashMap::new(),
            execution_time_ms: duration.as_millis() as u64,
        })
    }

    fn can_handle(&self, _mode: &CallMode) -> bool {
        true
    }
}

pub struct UnifiedCallService {
    hybrid_handler: HybridHandler,
}

impl UnifiedCallService {
    pub fn new() -> Self {
        Self {
            hybrid_handler: HybridHandler::new(),
        }
    }

    pub fn with_hybrid_handler(hybrid_handler: HybridHandler) -> Self {
        Self { hybrid_handler }
    }

    pub async fn call(&self, request: UnifiedCallRequest) -> Result<UnifiedCallResponse> {
        self.hybrid_handler.execute(request).await
    }

    pub async fn call_text(
        &self,
        text: String,
        options: Option<CallOptions>,
    ) -> Result<UnifiedCallResponse> {
        self.call(UnifiedCallRequest {
            call_mode: CallMode::Text,
            payload: Value::String(text),
            options,
        })
        .await
    }
}

impl Default for UnifiedCallService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UnifiedSkill {
    metadata: SkillMetadata,
    call_service: UnifiedCallService,
}

impl UnifiedSkill {
    pub fn new(metadata: SkillMetadata) -> Self {
        Self {
            metadata,
            call_service: UnifiedCallService::new(),
        }
    }

    pub fn with_call_service(mut self, call_service: UnifiedCallService) -> Self {
        self.call_service = call_service;
        self
    }
}

#[async_trait]
impl Skill for UnifiedSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let request = UnifiedCallRequest {
            call_mode: self.metadata.call_mode.clone(),
            payload: input,
            options: None,
        };

        let response = self.call_service.call(request).await?;

        if response.success {
            Ok(response.data.unwrap_or(Value::Null))
        } else {
            Err(crate::utils::AetherisError::Skill(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}
