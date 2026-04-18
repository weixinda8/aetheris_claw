use super::*;
use dashmap::DashMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceCase {
    pub id: String,
    pub title: String,
    pub description: String,
    pub device_ids: Vec<String>,
    pub fault_ids: Vec<String>,
    pub solution_ids: Vec<String>,
    pub tags: Vec<String>,
    pub resolution_summary: Option<String>,
    pub root_cause: Option<String>,
    pub duration_minutes: Option<u32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MaintenanceCase {
    pub fn new(title: String, description: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description,
            device_ids: Vec::new(),
            fault_ids: Vec::new(),
            solution_ids: Vec::new(),
            tags: Vec::new(),
            resolution_summary: None,
            root_cause: None,
            duration_minutes: None,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn with_device(mut self, device_id: String) -> Self {
        self.device_ids.push(device_id);
        self
    }

    pub fn with_fault(mut self, fault_id: String) -> Self {
        self.fault_ids.push(fault_id);
        self
    }

    pub fn with_solution(mut self, solution_id: String) -> Self {
        self.solution_ids.push(solution_id);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn similarity_score(&self, other: &MaintenanceCase) -> f64 {
        let mut score = 0.0;

        let device_overlap: Vec<_> = self
            .device_ids
            .iter()
            .filter(|id| other.device_ids.contains(id))
            .collect();
        score += device_overlap.len() as f64 * 0.3;

        let fault_overlap: Vec<_> = self
            .fault_ids
            .iter()
            .filter(|id| other.fault_ids.contains(id))
            .collect();
        score += fault_overlap.len() as f64 * 0.4;

        let tag_overlap: Vec<_> = self
            .tags
            .iter()
            .filter(|tag| other.tags.contains(tag))
            .collect();
        score += tag_overlap.len() as f64 * 0.3;

        score.min(1.0)
    }
}

pub struct CaseLibrary {
    cases: DashMap<String, MaintenanceCase>,
    tag_index: DashMap<String, Vec<String>>,
}

impl CaseLibrary {
    pub fn new() -> Self {
        Self {
            cases: DashMap::new(),
            tag_index: DashMap::new(),
        }
    }

    pub fn add_case(&mut self, case: MaintenanceCase) -> crate::utils::Result<String> {
        let case_id = case.id.clone();

        for tag in &case.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(case_id.clone());
        }

        self.cases.insert(case_id.clone(), case);
        Ok(case_id)
    }

    pub fn get_case(&self, case_id: &str) -> Option<MaintenanceCase> {
        self.cases.get(case_id).map(|entry| entry.value().clone())
    }

    pub fn update_case(&mut self, mut case: MaintenanceCase) -> crate::utils::Result<()> {
        case.updated_at = chrono::Utc::now();

        if let Some(old_case) = self.cases.get(&case.id) {
            for tag in &old_case.tags {
                if let Some(mut case_ids) = self.tag_index.get_mut(tag) {
                    case_ids.retain(|id| id != &case.id);
                }
            }
        }

        for tag in &case.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(case.id.clone());
        }

        self.cases.insert(case.id.clone(), case);
        Ok(())
    }

    pub fn delete_case(&mut self, case_id: &str) -> crate::utils::Result<bool> {
        if let Some((_, case)) = self.cases.remove(case_id) {
            for tag in &case.tags {
                if let Some(mut case_ids) = self.tag_index.get_mut(tag) {
                    case_ids.retain(|id| id != case_id);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn list_cases(&self, limit: Option<usize>) -> Vec<MaintenanceCase> {
        let mut cases: Vec<_> = self
            .cases
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        cases.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = limit {
            cases.truncate(limit);
        }

        cases
    }

    pub fn search_cases_by_tags(&self, tags: &[String]) -> Vec<MaintenanceCase> {
        let mut case_ids = std::collections::HashSet::new();

        for tag in tags {
            if let Some(tagged_case_ids) = self.tag_index.get(tag) {
                for id in tagged_case_ids.value() {
                    case_ids.insert(id.clone());
                }
            }
        }

        case_ids.iter().filter_map(|id| self.get_case(id)).collect()
    }

    pub fn search_cases_by_text(&self, query: &str) -> Vec<MaintenanceCase> {
        let query_lower = query.to_lowercase();
        self.cases
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|case| {
                case.title.to_lowercase().contains(&query_lower)
                    || case.description.to_lowercase().contains(&query_lower)
                    || case
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn find_similar_cases(
        &self,
        reference_case: &MaintenanceCase,
        top_k: usize,
    ) -> Vec<(MaintenanceCase, f64)> {
        let mut scored_cases: Vec<_> = self
            .cases
            .iter()
            .filter(|entry| entry.key() != &reference_case.id)
            .map(|entry| {
                let case = entry.value().clone();
                let score = reference_case.similarity_score(&case);
                (case, score)
            })
            .collect();

        scored_cases.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_cases.truncate(top_k);
        scored_cases
    }
}

impl Default for CaseLibrary {
    fn default() -> Self {
        Self::new()
    }
}
