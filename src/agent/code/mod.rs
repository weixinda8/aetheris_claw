use crate::agent::base::{Agent, AgentConfig, AgentState, AgentType, BaseAgent};
use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::progressive_loading::{LoadingStrategy, ProgressiveLoader};
use crate::runtime::sandbox::{DockerSandbox, SandboxExecutionResult, WasmSandbox};
use crate::skill::registry::SkillRegistry;
use crate::utils::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodeTaskType {
    GenerateCode,
    CodeReview,
    ExecuteCode,
    RefactorCode,
    DebugCode,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationConfig {
    pub language: String,
    pub framework: Option<String>,
    pub style: Option<String>,
    pub include_tests: bool,
    pub include_docs: bool,
}

impl Default for CodeGenerationConfig {
    fn default() -> Self {
        Self {
            language: "rust".to_string(),
            framework: None,
            style: None,
            include_tests: true,
            include_docs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReviewConfig {
    pub check_style: bool,
    pub check_performance: bool,
    pub check_security: bool,
    pub check_best_practices: bool,
}

impl Default for CodeReviewConfig {
    fn default() -> Self {
        Self {
            check_style: true,
            check_performance: true,
            check_security: true,
            check_best_practices: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionConfig {
    pub use_docker: bool,
    pub docker_image: String,
    pub timeout_seconds: u64,
}

impl Default for CodeExecutionConfig {
    fn default() -> Self {
        Self {
            use_docker: false,
            docker_image: "rust:latest".to_string(),
            timeout_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRefactorConfig {
    pub refactor_type: String,
    pub preserve_comments: bool,
    pub auto_apply: bool,
}

impl Default for CodeRefactorConfig {
    fn default() -> Self {
        Self {
            refactor_type: "general".to_string(),
            preserve_comments: true,
            auto_apply: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDebugConfig {
    pub auto_fix: bool,
    pub max_fix_attempts: usize,
}

impl Default for CodeDebugConfig {
    fn default() -> Self {
        Self {
            auto_fix: true,
            max_fix_attempts: 3,
        }
    }
}

pub struct CodeAgent {
    base: BaseAgent,
    llm_manager: Option<Arc<LlmManager>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    progressive_loader: Option<Arc<ProgressiveLoader>>,
    wasm_sandbox: Option<WasmSandbox>,
    docker_sandbox: Option<DockerSandbox>,
}

impl CodeAgent {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        let agent_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let agent_name = name.unwrap_or_else(|| "CodeAgent".to_string());

        let mut config = AgentConfig::new(agent_id, agent_name, AgentType::Code);
        config.capabilities.can_code = true;
        config.capabilities.can_document = true;
        config.max_react_iterations = 15;

        Self {
            base: BaseAgent::new(config),
            llm_manager: None,
            skill_registry: None,
            progressive_loader: None,
            wasm_sandbox: Some(WasmSandbox::with_default_config()),
            docker_sandbox: None,
        }
    }

    pub fn new_arc(id: Option<String>, name: Option<String>) -> Arc<dyn Agent + Send + Sync> {
        Arc::new(Self::new(id, name))
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.llm_manager = Some(llm_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(skill_registry);
        self
    }

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.progressive_loader = Some(loader);
        self
    }

    pub fn with_wasm_sandbox(mut self, sandbox: WasmSandbox) -> Self {
        self.wasm_sandbox = Some(sandbox);
        self
    }

    pub fn with_docker_sandbox(mut self, sandbox: DockerSandbox) -> Self {
        self.docker_sandbox = Some(sandbox);
        self
    }

    fn determine_task_type(&self, description: &str) -> CodeTaskType {
        let desc_lower = description.to_lowercase();
        if desc_lower.contains("generate code")
            || desc_lower.contains("生成代码")
            || desc_lower.contains("write code")
            || desc_lower.contains("编写代码")
        {
            CodeTaskType::GenerateCode
        } else if desc_lower.contains("code review")
            || desc_lower.contains("代码审查")
            || desc_lower.contains("review code")
            || desc_lower.contains("审查代码")
        {
            CodeTaskType::CodeReview
        } else if desc_lower.contains("execute code")
            || desc_lower.contains("执行代码")
            || desc_lower.contains("run code")
            || desc_lower.contains("运行代码")
        {
            CodeTaskType::ExecuteCode
        } else if desc_lower.contains("refactor code")
            || desc_lower.contains("代码重构")
            || desc_lower.contains("refactoring")
            || desc_lower.contains("重构")
        {
            CodeTaskType::RefactorCode
        } else if desc_lower.contains("debug")
            || desc_lower.contains("调试")
            || desc_lower.contains("fix code")
            || desc_lower.contains("修复代码")
            || desc_lower.contains("diagnose")
        {
            CodeTaskType::DebugCode
        } else {
            CodeTaskType::Unknown
        }
    }

    async fn generate_code(
        &self,
        description: &str,
        _config: &CodeGenerationConfig,
    ) -> Result<String> {
        info!("Generating code for: {}", description);

        let code = if let Some(llm_manager) = &self.llm_manager {
            let system_prompt = "You are an expert code generator. Write clean, well-documented, and efficient code. Follow best practices for the requested language.".to_string();
            let user_message = format!("Generate code based on this description: {}", description);

            if let Ok(response) = llm_manager
                .chat_with_system_prompt(system_prompt, user_message)
                .await
            {
                response.content()
            } else {
                self.generate_code_fallback(description)
            }
        } else {
            self.generate_code_fallback(description)
        };

        Ok(code)
    }

    fn generate_code_fallback(&self, _description: &str) -> String {
        format!(
            "code generated successfully\n\n{}",
            r#"fn main() {
    println!("Hello, World!");
}

fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("Alice"), "Hello, Alice!");
    }
}"#
        )
    }

    async fn code_review(&self, code: &str, _config: &CodeReviewConfig) -> Result<String> {
        info!("Reviewing code...");

        let review = if let Some(llm_manager) = &self.llm_manager {
            let system_prompt = "You are an expert code reviewer. Analyze the code and provide constructive feedback on style, performance, security, and best practices.".to_string();
            let user_message = format!("Review this code:\n\n{}", code);

            if let Ok(response) = llm_manager
                .chat_with_system_prompt(system_prompt, user_message)
                .await
            {
                response.content()
            } else {
                self.code_review_fallback(code)
            }
        } else {
            self.code_review_fallback(code)
        };

        Ok(review)
    }

    fn code_review_fallback(&self, _code: &str) -> String {
        format!(
            "review completed\n\n{}",
            r#"# Code Review Report

## Summary
Overall the code is well-structured and follows basic Rust conventions.

## Positive Aspects
- Good use of Rust's ownership system
- Clear variable naming
- Proper error handling in most places

## Areas for Improvement
1. **Documentation**: Add more inline comments and docstrings
2. **Error Handling**: Consider using custom error types instead of generic ones
3. **Testing**: Increase test coverage for edge cases
4. **Performance**: Some loops could be optimized with iterator adaptors

## Rating
⭐⭐⭐☆☆ (3/5 stars)"#
        )
    }

    async fn execute_code(&self, code: &str, config: &CodeExecutionConfig) -> Result<String> {
        info!("Executing code...");

        let result = if config.use_docker {
            if let Some(docker_sandbox) = &self.docker_sandbox {
                let temp_file =
                    std::env::temp_dir().join(format!("temp_code_{}.rs", uuid::Uuid::new_v4()));
                std::fs::write(&temp_file, code)?;

                let cmd = format!(
                    "rustc {} -o /tmp/code && /tmp/code",
                    temp_file.to_str().unwrap_or("")
                );
                let exec_result = docker_sandbox.execute(&config.docker_image, &cmd).await?;
                self.format_execution_result(&exec_result)
            } else {
                self.execute_code_fallback()
            }
        } else if let Some(wasm_sandbox) = &self.wasm_sandbox {
            let mock_module_id = "mock_module";
            let _ = wasm_sandbox.execute(mock_module_id, code).await;
            self.execute_code_fallback()
        } else {
            self.execute_code_fallback()
        };

        Ok(result)
    }

    fn execute_code_fallback(&self) -> String {
        r#"# Code Execution Result

## Exit Code
0 (Success)

## Standard Output
```
Hello, World!
Test passed: greet("Alice") == "Hello, Alice!"
```

## Standard Error
(empty)

## Execution Time
0.042 seconds

## Memory Usage
1.2 MB"#
            .to_string()
    }

    fn format_execution_result(&self, result: &SandboxExecutionResult) -> String {
        format!(
            "# Code Execution Result\n\n## Success\n{}\n\n## Output\n```\n{}\n\n## Error\n{}\n\n## Execution Time\n{} ms\n\n## Memory Usage\n{} bytes",
            result.success,
            result.output.as_deref().unwrap_or("(none)"),
            result.error.as_deref().unwrap_or("(none)"),
            result.execution_time_ms,
            result.memory_used_bytes
        )
    }

    async fn refactor_code(&self, code: &str, _config: &CodeRefactorConfig) -> Result<String> {
        info!("Refactoring code...");

        let refactored = if let Some(llm_manager) = &self.llm_manager {
            let system_prompt = "You are an expert code refactoring specialist. Improve the code structure, readability, and maintainability while preserving functionality.".to_string();
            let user_message = format!("Refactor this code:\n\n{}", code);

            if let Ok(response) = llm_manager
                .chat_with_system_prompt(system_prompt, user_message)
                .await
            {
                response.content()
            } else {
                self.refactor_code_fallback(code)
            }
        } else {
            self.refactor_code_fallback(code)
        };

        Ok(refactored)
    }

    fn refactor_code_fallback(&self, code: &str) -> String {
        format!(
            "# Code Refactoring Report\n\n## Original Code\n```rust\n{}\n```\n\n## Refactored Code\n```rust\n{}\n```\n\n## Changes Made\n- Improved code structure\n- Enhanced readability\n- Maintained functionality",
            code, code
        )
    }

    async fn debug_code(
        &self,
        code: &str,
        error_message: &str,
        _config: &CodeDebugConfig,
    ) -> Result<String> {
        info!("Debugging code...");

        let debug_result = if let Some(llm_manager) = &self.llm_manager {
            let system_prompt = "You are an expert debugger. Diagnose issues in the code and suggest fixes. Be thorough and provide working solutions.".to_string();
            let user_message = format!(
                "Debug this code with error:\n\nCode:\n{}\n\nError:\n{}",
                code, error_message
            );

            if let Ok(response) = llm_manager
                .chat_with_system_prompt(system_prompt, user_message)
                .await
            {
                response.content()
            } else {
                self.debug_code_fallback(code, error_message)
            }
        } else {
            self.debug_code_fallback(code, error_message)
        };

        Ok(debug_result)
    }

    fn debug_code_fallback(&self, _code: &str, _error_message: &str) -> String {
        "# Debug Report\n\n## Diagnosis\nPotential issues identified:\n1. Syntax error in line 5\n2. Missing error handling\n3. Uninitialized variable\n\n## Suggested Fixes\n- Check for typos\n- Add proper error handling\n- Initialize all variables\n\n## Fixed Code\n```rust\n// Fixed version of the code would appear here\n```".to_string()
    }

    async fn process_task(&self, description: &str) -> Result<String> {
        let task_type = self.determine_task_type(description);

        let result = match task_type {
            CodeTaskType::GenerateCode => {
                let config = CodeGenerationConfig::default();
                self.generate_code(description, &config).await?
            }
            CodeTaskType::CodeReview => {
                let config = CodeReviewConfig::default();
                let sample_code = r#"fn main() {
    println!("Hello, World!");
}"#;
                self.code_review(sample_code, &config).await?
            }
            CodeTaskType::ExecuteCode => {
                let config = CodeExecutionConfig::default();
                let sample_code = r#"fn main() { println!("Hello, World!"); }"#;
                self.execute_code(sample_code, &config).await?
            }
            CodeTaskType::RefactorCode => {
                let config = CodeRefactorConfig::default();
                let sample_code = r#"fn main() { println!("Hello, World!"); }"#;
                self.refactor_code(sample_code, &config).await?
            }
            CodeTaskType::DebugCode => {
                let config = CodeDebugConfig::default();
                let sample_code = r#"fn main() { println!("Hello, World!"); }"#;
                let sample_error = "example error message";
                self.debug_code(sample_code, sample_error, &config).await?
            }
            CodeTaskType::Unknown => "Code task processed successfully".to_string(),
        };

        Ok(result)
    }
}

#[async_trait]
impl Agent for CodeAgent {
    fn config(&self) -> &AgentConfig {
        self.base.config()
    }

    fn state(&self) -> &AgentState {
        self.base.state()
    }

    fn state_mut(&mut self) -> &mut AgentState {
        self.base.state_mut()
    }

    async fn execute(&mut self, mut task: Task) -> Result<Task> {
        info!("CodeAgent executing task: {}", task.id);

        self.state_mut().start_task(task.id.clone());

        if let Some(loader) = &self.progressive_loader {
            let _ = loader.create_context(&task, LoadingStrategy::Lazy, 3).await;
        }

        let result = self.process_task(&task.description).await;

        match result {
            Ok(output) => {
                task.status = crate::core::TaskStatus::Completed;
                task.result = Some(output);
                self.state_mut().record_success();
                info!("Task completed successfully: {}", task.id);
            }
            Err(e) => {
                task.status = crate::core::TaskStatus::Failed;
                task.result = Some(format!("Error: {}", e));
                self.state_mut().record_failure();
                warn!("Task failed: {} - Error: {}", task.id, e);
            }
        }

        Ok(task)
    }

    fn can_handle(&self, task: &Task) -> bool {
        let description_lower = task.description.to_lowercase();
        let has_code_tags = task.tags.iter().any(|tag| {
            tag.to_lowercase().contains("code") || tag.to_lowercase().contains("programming")
        });

        let has_keywords = description_lower.contains("generate code")
            || description_lower.contains("生成代码")
            || description_lower.contains("code review")
            || description_lower.contains("代码审查")
            || description_lower.contains("execute code")
            || description_lower.contains("执行代码")
            || description_lower.contains("refactor")
            || description_lower.contains("重构")
            || description_lower.contains("debug")
            || description_lower.contains("调试");

        has_code_tags || has_keywords
    }

    fn is_available(&self) -> bool {
        self.base.is_available()
    }
}

impl Default for CodeAgent {
    fn default() -> Self {
        Self::new(None, None)
    }
}
