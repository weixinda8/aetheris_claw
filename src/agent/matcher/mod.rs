use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub current_load: f64,
    pub performance_score: f64,
    pub location: Option<String>,
    pub cost_per_minute: f64,
    pub availability: f64,
    pub metadata: Option<serde_json::Value>,
}

impl Default for AgentProfile {
    fn default() -> Self {
        Self {
            agent_id: Uuid::new_v4().to_string(),
            name: String::new(),
            capabilities: Vec::new(),
            tags: Vec::new(),
            current_load: 0.0,
            performance_score: 0.5,
            location: None,
            cost_per_minute: 0.0,
            availability: 1.0,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirement {
    pub required_capabilities: Vec<String>,
    pub preferred_capabilities: Vec<String>,
    pub required_tags: Vec<String>,
    pub preferred_tags: Vec<String>,
    pub priority: u8,
    pub estimated_duration_minutes: u32,
    pub max_cost: Option<f64>,
    pub preferred_locations: Option<Vec<String>>,
    pub load_balance_weight: Option<f64>,
    pub performance_weight: Option<f64>,
    pub cost_weight: Option<f64>,
    pub location_weight: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

impl Default for TaskRequirement {
    fn default() -> Self {
        Self {
            required_capabilities: Vec::new(),
            preferred_capabilities: Vec::new(),
            required_tags: Vec::new(),
            preferred_tags: Vec::new(),
            priority: 0,
            estimated_duration_minutes: 0,
            max_cost: None,
            preferred_locations: None,
            load_balance_weight: Some(0.2),
            performance_weight: Some(0.4),
            cost_weight: Some(0.2),
            location_weight: Some(0.2),
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMatch {
    pub agent: AgentProfile,
    pub total_score: f64,
    pub capability_score: f64,
    pub load_score: f64,
    pub performance_score: f64,
    pub cost_score: f64,
    pub location_score: f64,
    pub matched_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub is_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub task_requirement: TaskRequirement,
    pub matches: Vec<AgentMatch>,
    pub best_match: Option<AgentMatch>,
    pub fallback_agents: Vec<AgentMatch>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStrategy {
    pub use_fallback: bool,
    pub fallback_capabilities: Vec<String>,
    pub max_fallback_agents: usize,
    pub fallback_score_threshold: f64,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self {
            use_fallback: true,
            fallback_capabilities: Vec::new(),
            max_fallback_agents: 3,
            fallback_score_threshold: 0.3,
        }
    }
}

#[async_trait]
pub trait AgentMatcher: Send + Sync {
    async fn match_agents(
        &self,
        requirement: TaskRequirement,
        agents: &[AgentProfile],
    ) -> crate::utils::Result<MatchResult>;

    async fn rank_agents(
        &self,
        requirement: &TaskRequirement,
        agents: &[AgentProfile],
    ) -> crate::utils::Result<Vec<AgentMatch>>;

    fn calculate_capability_score(
        &self,
        requirement: &TaskRequirement,
        agent: &AgentProfile,
    ) -> (f64, Vec<String>, Vec<String>);

    fn calculate_load_score(&self, agent: &AgentProfile) -> f64;

    fn calculate_performance_score(&self, agent: &AgentProfile) -> f64;

    fn calculate_cost_score(&self, requirement: &TaskRequirement, agent: &AgentProfile) -> f64;

    fn calculate_location_score(&self, requirement: &TaskRequirement, agent: &AgentProfile) -> f64;

    fn set_fallback_strategy(&mut self, strategy: FallbackStrategy);

    fn get_fallback_strategy(&self) -> &FallbackStrategy;
}

pub struct SmartAgentMatcher {
    fallback_strategy: FallbackStrategy,
}

impl SmartAgentMatcher {
    pub fn new() -> Self {
        Self {
            fallback_strategy: FallbackStrategy::default(),
        }
    }

    fn calculate_total_score(
        &self,
        requirement: &TaskRequirement,
        capability_score: f64,
        load_score: f64,
        performance_score: f64,
        cost_score: f64,
        location_score: f64,
    ) -> f64 {
        let load_weight = requirement.load_balance_weight.unwrap_or(0.2);
        let perf_weight = requirement.performance_weight.unwrap_or(0.4);
        let cost_weight = requirement.cost_weight.unwrap_or(0.2);
        let loc_weight = requirement.location_weight.unwrap_or(0.2);

        (capability_score * (1.0 - load_weight - perf_weight - cost_weight - loc_weight))
            + (load_score * load_weight)
            + (performance_score * perf_weight)
            + (cost_score * cost_weight)
            + (location_score * loc_weight)
    }

    fn find_fallback_agents(
        &self,
        requirement: &TaskRequirement,
        agents: &[AgentProfile],
    ) -> Vec<AgentMatch> {
        if !self.fallback_strategy.use_fallback {
            return Vec::new();
        }

        let mut fallback_requirement = requirement.clone();
        fallback_requirement.required_capabilities =
            self.fallback_strategy.fallback_capabilities.clone();
        fallback_requirement.required_tags = Vec::new();

        let mut fallback_matches = Vec::new();

        for agent in agents {
            let (cap_score, matched, missing) =
                self.calculate_capability_score(&fallback_requirement, agent);

            if cap_score >= self.fallback_strategy.fallback_score_threshold {
                let load_score = self.calculate_load_score(agent);
                let perf_score = self.calculate_performance_score(agent);
                let cost_score = self.calculate_cost_score(requirement, agent);
                let loc_score = self.calculate_location_score(requirement, agent);

                let total_score = self.calculate_total_score(
                    requirement,
                    cap_score,
                    load_score,
                    perf_score,
                    cost_score,
                    loc_score,
                );

                fallback_matches.push(AgentMatch {
                    agent: agent.clone(),
                    total_score,
                    capability_score: cap_score,
                    load_score,
                    performance_score: perf_score,
                    cost_score,
                    location_score: loc_score,
                    matched_capabilities: matched,
                    missing_capabilities: missing,
                    is_fallback: true,
                });
            }
        }

        fallback_matches.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());
        fallback_matches
            .into_iter()
            .take(self.fallback_strategy.max_fallback_agents)
            .collect()
    }
}

#[async_trait]
impl AgentMatcher for SmartAgentMatcher {
    async fn match_agents(
        &self,
        requirement: TaskRequirement,
        agents: &[AgentProfile],
    ) -> crate::utils::Result<MatchResult> {
        let matches = self.rank_agents(&requirement, agents).await?;

        let best_match = matches.first().cloned();
        let fallback_agents = self.find_fallback_agents(&requirement, agents);

        Ok(MatchResult {
            task_requirement: requirement,
            matches,
            best_match,
            fallback_agents,
            created_at: chrono::Utc::now(),
        })
    }

    async fn rank_agents(
        &self,
        requirement: &TaskRequirement,
        agents: &[AgentProfile],
    ) -> crate::utils::Result<Vec<AgentMatch>> {
        let mut matches = Vec::new();

        for agent in agents {
            let (cap_score, matched, missing) = self.calculate_capability_score(requirement, agent);

            if cap_score < 1.0 && !requirement.required_capabilities.is_empty() {
                continue;
            }

            if let Some(max_cost) = requirement.max_cost {
                let estimated_cost =
                    agent.cost_per_minute * requirement.estimated_duration_minutes as f64;
                if estimated_cost > max_cost {
                    continue;
                }
            }

            let has_required_tags = requirement
                .required_tags
                .iter()
                .all(|tag| agent.tags.contains(tag));
            if !has_required_tags && !requirement.required_tags.is_empty() {
                continue;
            }

            let load_score = self.calculate_load_score(agent);
            let perf_score = self.calculate_performance_score(agent);
            let cost_score = self.calculate_cost_score(requirement, agent);
            let loc_score = self.calculate_location_score(requirement, agent);

            let total_score = self.calculate_total_score(
                requirement,
                cap_score,
                load_score,
                perf_score,
                cost_score,
                loc_score,
            );

            matches.push(AgentMatch {
                agent: agent.clone(),
                total_score,
                capability_score: cap_score,
                load_score,
                performance_score: perf_score,
                cost_score,
                location_score: loc_score,
                matched_capabilities: matched,
                missing_capabilities: missing,
                is_fallback: false,
            });
        }

        matches.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());

        Ok(matches)
    }

    fn calculate_capability_score(
        &self,
        requirement: &TaskRequirement,
        agent: &AgentProfile,
    ) -> (f64, Vec<String>, Vec<String>) {
        let agent_caps: HashSet<_> = agent.capabilities.iter().cloned().collect();
        let required_caps: HashSet<_> = requirement.required_capabilities.iter().cloned().collect();
        let preferred_caps: HashSet<_> =
            requirement.preferred_capabilities.iter().cloned().collect();

        let matched_required: Vec<_> = agent_caps.intersection(&required_caps).cloned().collect();
        let missing_required: Vec<_> = required_caps.difference(&agent_caps).cloned().collect();
        let matched_preferred: Vec<_> = agent_caps.intersection(&preferred_caps).cloned().collect();

        let required_score = if required_caps.is_empty() {
            1.0
        } else {
            matched_required.len() as f64 / required_caps.len() as f64
        };

        let preferred_score = if preferred_caps.is_empty() {
            1.0
        } else {
            matched_preferred.len() as f64 / preferred_caps.len() as f64
        };

        let total_score = (required_score * 0.7) + (preferred_score * 0.3);
        let all_matched: Vec<_> = matched_required
            .into_iter()
            .chain(matched_preferred)
            .collect();

        (total_score, all_matched, missing_required)
    }

    fn calculate_load_score(&self, agent: &AgentProfile) -> f64 {
        1.0 - agent.current_load.min(1.0)
    }

    fn calculate_performance_score(&self, agent: &AgentProfile) -> f64 {
        agent.performance_score.clamp(0.0, 1.0)
    }

    fn calculate_cost_score(&self, requirement: &TaskRequirement, agent: &AgentProfile) -> f64 {
        if let Some(max_cost) = requirement.max_cost {
            let estimated_cost =
                agent.cost_per_minute * requirement.estimated_duration_minutes as f64;
            if estimated_cost <= 0.0 {
                return 1.0;
            }
            (1.0 - (estimated_cost / max_cost)).max(0.0)
        } else {
            1.0 - (agent.cost_per_minute / 100.0).min(1.0)
        }
    }

    fn calculate_location_score(&self, requirement: &TaskRequirement, agent: &AgentProfile) -> f64 {
        if let Some(preferred_locs) = &requirement.preferred_locations {
            if preferred_locs.is_empty() {
                return 1.0;
            }
            if let Some(agent_loc) = &agent.location {
                if preferred_locs.contains(agent_loc) {
                    return 1.0;
                }
            }
            0.0
        } else {
            1.0
        }
    }

    fn set_fallback_strategy(&mut self, strategy: FallbackStrategy) {
        self.fallback_strategy = strategy;
    }

    fn get_fallback_strategy(&self) -> &FallbackStrategy {
        &self.fallback_strategy
    }
}

impl Default for SmartAgentMatcher {
    fn default() -> Self {
        Self::new()
    }
}
