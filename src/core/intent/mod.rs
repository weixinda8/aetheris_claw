use crate::core::Task;
use crate::core::llm::LlmManager;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    Security,
    Compliance,
    Permission,
    Resource,
    Time,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub description: String,
    pub is_mandatory: bool,
    pub validation_rules: Vec<String>,
}

impl Constraint {
    pub fn new(constraint_type: ConstraintType, description: String, is_mandatory: bool) -> Self {
        Self {
            constraint_type,
            description,
            is_mandatory,
            validation_rules: Vec::new(),
        }
    }

    pub fn with_validation_rules(mut self, rules: Vec<String>) -> Self {
        self.validation_rules = rules;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntentConfidence {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: String,
    pub raw_input: String,
    pub parsed_goal: String,
    pub constraints: Vec<Constraint>,
    pub requirements: Vec<String>,
    pub confidence: IntentConfidence,
    pub missing_information: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Intent {
    pub fn new(raw_input: String) -> Self {
        Self {
            intent_id: uuid::Uuid::new_v4().to_string(),
            raw_input,
            parsed_goal: String::new(),
            constraints: Vec::new(),
            requirements: Vec::new(),
            confidence: IntentConfidence::Low,
            missing_information: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_parsed_goal(mut self, goal: String) -> Self {
        self.parsed_goal = goal;
        self
    }

    pub fn with_confidence(mut self, confidence: IntentConfidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn add_requirement(&mut self, requirement: String) {
        self.requirements.push(requirement);
    }

    pub fn add_missing_information(&mut self, info: String) {
        self.missing_information.push(info);
    }

    pub fn has_missing_information(&self) -> bool {
        !self.missing_information.is_empty()
    }

    pub fn is_executable(&self) -> bool {
        !self.parsed_goal.is_empty()
            && self.confidence >= IntentConfidence::Medium
            && !self.has_missing_information()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub violated_constraints: Vec<Constraint>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            violated_constraints: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(violated: Vec<Constraint>) -> Self {
        Self {
            is_valid: false,
            violated_constraints: violated,
            warnings: Vec::new(),
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

type ConstraintRuleFn = Box<dyn Fn(&Constraint) -> bool + Send + Sync>;
type ConstraintRules = Arc<DashMap<ConstraintType, Vec<ConstraintRuleFn>>>;

pub struct ConstraintValidator {
    rules: ConstraintRules,
}

impl ConstraintValidator {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub fn register_rule<F>(&self, constraint_type: ConstraintType, rule: F)
    where
        F: Fn(&Constraint) -> bool + Send + Sync + 'static,
    {
        self.rules
            .entry(constraint_type)
            .or_default()
            .push(Box::new(rule));
    }

    pub fn validate_constraints(&self, constraints: &[Constraint]) -> ValidationResult {
        let mut violated = Vec::new();
        let mut warnings = Vec::new();

        for constraint in constraints {
            let is_valid = self.validate_single_constraint(constraint);
            if !is_valid && constraint.is_mandatory {
                violated.push(constraint.clone());
            } else if !is_valid {
                warnings.push(format!(
                    "Non-mandatory constraint violated: {}",
                    constraint.description
                ));
            }
        }

        let mut result = if violated.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(violated)
        };

        result.warnings = warnings;
        result
    }

    fn validate_single_constraint(&self, constraint: &Constraint) -> bool {
        if let Some(rules) = self.rules.get(&constraint.constraint_type) {
            for rule in rules.iter() {
                if !rule(constraint) {
                    return false;
                }
            }
        }
        true
    }
}

impl Default for ConstraintValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IntentParser {
    constraint_validator: Arc<ConstraintValidator>,
    common_constraints: Vec<Constraint>,
    llm_manager: Option<Arc<LlmManager>>,
}

impl IntentParser {
    pub fn new() -> Self {
        Self {
            constraint_validator: Arc::new(ConstraintValidator::new()),
            common_constraints: Vec::new(),
            llm_manager: None,
        }
    }

    pub fn with_constraint_validator(mut self, validator: ConstraintValidator) -> Self {
        self.constraint_validator = Arc::new(validator);
        self
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.llm_manager = Some(llm_manager);
        self
    }

    pub fn add_common_constraint(&mut self, constraint: Constraint) {
        self.common_constraints.push(constraint);
    }

    pub async fn parse(&self, input: &str) -> Result<Intent> {
        info!("Parsing intent from input: {}", input);

        let mut intent = Intent::new(input.to_string());

        if let Some(llm_manager) = &self.llm_manager {
            intent = self.parse_with_llm(intent, input, llm_manager).await?;
        } else {
            intent = self.extract_goal(intent, input)?;
            intent = self.extract_constraints(intent, input)?;
            intent = self.extract_requirements(intent, input)?;
            intent = self.detect_missing_information(intent)?;
            intent = self.calculate_confidence(intent)?;
        }

        for constraint in &self.common_constraints {
            intent.add_constraint(constraint.clone());
        }

        debug!("Parsed intent: {:?}", intent);
        Ok(intent)
    }

    async fn parse_with_llm(
        &self,
        mut intent: Intent,
        input: &str,
        llm_manager: &Arc<LlmManager>,
    ) -> Result<Intent> {
        info!("Parsing intent with LLM");

        let system_prompt = r#"You are an intent parser for a task execution system. Your job is to analyze user input and extract structured information.

Please respond with a JSON object in the following format:
{
  "goal": "clear, concise statement of the user's goal",
  "constraints": [
    {
      "type": "Security|Compliance|Permission|Resource|Time|Custom",
      "description": "description of the constraint",
      "is_mandatory": true|false
    }
  ],
  "requirements": ["list of specific requirements"],
  "missing_information": ["list of missing information needed to execute the task"],
  "confidence_score": 0-100
}

Guidelines:
- goal: Extract the main objective in 1-2 sentences
- constraints: Identify security, compliance, time, or resource constraints
- requirements: List specific things the user needs or wants
- missing_information: What information is needed but not provided
- confidence_score: 0-100 based on how clear and complete the input is

Only respond with the JSON, no other text."#.to_string();

        let response = llm_manager
            .chat_with_system_prompt(system_prompt, input.to_string())
            .await;

        match response {
            Ok(chat_response) => {
                if let Some(choice) = chat_response.choices.first() {
                    let content = &choice.message.content;
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                        if let Some(goal) = parsed.get("goal").and_then(|g| g.as_str()) {
                            intent.parsed_goal = goal.to_string();
                        }

                        if let Some(constraints) =
                            parsed.get("constraints").and_then(|c| c.as_array())
                        {
                            for constraint in constraints {
                                if let (Some(typ), Some(desc), Some(mandatory)) = (
                                    constraint.get("type").and_then(|t| t.as_str()),
                                    constraint.get("description").and_then(|d| d.as_str()),
                                    constraint.get("is_mandatory").and_then(|m| m.as_bool()),
                                ) {
                                    let constraint_type = match typ.to_lowercase().as_str() {
                                        "security" => ConstraintType::Security,
                                        "compliance" => ConstraintType::Compliance,
                                        "permission" => ConstraintType::Permission,
                                        "resource" => ConstraintType::Resource,
                                        "time" => ConstraintType::Time,
                                        _ => ConstraintType::Custom,
                                    };
                                    intent.add_constraint(Constraint::new(
                                        constraint_type,
                                        desc.to_string(),
                                        mandatory,
                                    ));
                                }
                            }
                        }

                        if let Some(requirements) =
                            parsed.get("requirements").and_then(|r| r.as_array())
                        {
                            for req in requirements {
                                if let Some(req_str) = req.as_str() {
                                    intent.add_requirement(req_str.to_string());
                                }
                            }
                        }

                        if let Some(missing) =
                            parsed.get("missing_information").and_then(|m| m.as_array())
                        {
                            for info in missing {
                                if let Some(info_str) = info.as_str() {
                                    intent.add_missing_information(info_str.to_string());
                                }
                            }
                        }

                        if let Some(score) = parsed.get("confidence_score").and_then(|s| s.as_u64())
                        {
                            intent.confidence = match score {
                                0..=30 => IntentConfidence::Low,
                                31..=50 => IntentConfidence::Medium,
                                51..=75 => IntentConfidence::High,
                                76..=100 => IntentConfidence::VeryHigh,
                                _ => IntentConfidence::Medium,
                            };
                        }
                    }
                }
            }
            Err(e) => {
                warn!("LLM parsing failed, falling back to rule-based: {}", e);
                intent = self.extract_goal(intent, input)?;
                intent = self.extract_constraints(intent, input)?;
                intent = self.extract_requirements(intent, input)?;
                intent = self.detect_missing_information(intent)?;
                intent = self.calculate_confidence(intent)?;
            }
        }

        Ok(intent)
    }

    fn extract_goal(&self, mut intent: Intent, input: &str) -> Result<Intent> {
        let goal = input.trim().to_string();
        intent.parsed_goal = if goal.len() > 500 {
            goal.chars().take(500).collect()
        } else {
            goal
        };
        Ok(intent)
    }

    fn extract_constraints(&self, mut intent: Intent, input: &str) -> Result<Intent> {
        let lower_input = input.to_lowercase();

        if lower_input.contains("secure") || lower_input.contains("安全") {
            intent.add_constraint(Constraint::new(
                ConstraintType::Security,
                "Security constraints required".to_string(),
                true,
            ));
        }

        if lower_input.contains("compliance") || lower_input.contains("合规") {
            intent.add_constraint(Constraint::new(
                ConstraintType::Compliance,
                "Compliance check required".to_string(),
                true,
            ));
        }

        if lower_input.contains("before")
            || lower_input.contains("by")
            || lower_input.contains("deadline")
        {
            intent.add_constraint(Constraint::new(
                ConstraintType::Time,
                "Time constraint detected".to_string(),
                false,
            ));
        }

        Ok(intent)
    }

    fn extract_requirements(&self, mut intent: Intent, input: &str) -> Result<Intent> {
        let keywords = ["need", "requires", "must", "should", "需要", "必须"];
        let lower_input = input.to_lowercase();

        for &keyword in &keywords {
            if lower_input.contains(keyword) {
                intent.add_requirement(format!("Requirement detected: {}", keyword));
            }
        }

        Ok(intent)
    }

    fn detect_missing_information(&self, mut intent: Intent) -> Result<Intent> {
        if intent.parsed_goal.is_empty() {
            intent.add_missing_information("Clear goal description".to_string());
        }

        if intent.parsed_goal.len() < 10 {
            intent.add_missing_information("More detailed description".to_string());
        }

        Ok(intent)
    }

    fn calculate_confidence(&self, mut intent: Intent) -> Result<Intent> {
        let mut score = 0;

        if !intent.parsed_goal.is_empty() {
            score += 30;
        }

        if intent.parsed_goal.len() >= 20 {
            score += 20;
        }

        if !intent.constraints.is_empty() {
            score += 20;
        }

        if !intent.requirements.is_empty() {
            score += 15;
        }

        if intent.missing_information.is_empty() {
            score += 15;
        }

        intent.confidence = match score {
            0..=30 => IntentConfidence::Low,
            31..=50 => IntentConfidence::Medium,
            51..=75 => IntentConfidence::High,
            76..=100 => IntentConfidence::VeryHigh,
            _ => IntentConfidence::Medium,
        };

        Ok(intent)
    }

    pub fn validate_intent(&self, intent: &Intent) -> Result<ValidationResult> {
        info!("Validating intent: {}", intent.intent_id);

        let constraint_result = self
            .constraint_validator
            .validate_constraints(&intent.constraints);

        let mut result = constraint_result;

        if !intent.is_executable() {
            result.is_valid = false;
            if intent.has_missing_information() {
                result.add_warning(format!(
                    "Missing information: {:?}",
                    intent.missing_information
                ));
            }
        }

        Ok(result)
    }

    pub fn to_task(&self, intent: Intent) -> Result<Task> {
        info!("Converting intent to task: {}", intent.intent_id);

        let validation = self.validate_intent(&intent)?;
        if !validation.is_valid {
            return Err(AetherisError::IntentValidation(format!(
                "Intent validation failed: {:?}",
                validation.violated_constraints
            )));
        }

        let priority = match intent.confidence {
            IntentConfidence::VeryHigh => 1,
            IntentConfidence::High => 3,
            IntentConfidence::Medium => 5,
            IntentConfidence::Low => 8,
        };

        let mut task = Task::new(intent.parsed_goal, priority);

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "intent_id".to_string(),
            serde_json::Value::String(intent.intent_id),
        );
        metadata.insert(
            "confidence".to_string(),
            serde_json::Value::String(format!("{:?}", intent.confidence)),
        );
        metadata.insert(
            "constraints".to_string(),
            serde_json::to_value(&intent.constraints)?,
        );
        task.metadata = serde_json::Value::Object(metadata);

        Ok(task)
    }

    pub fn ask_for_missing_info(&self, intent: &Intent) -> Vec<String> {
        intent.missing_information.clone()
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}
