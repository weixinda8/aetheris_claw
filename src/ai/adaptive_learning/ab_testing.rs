use super::*;
use dashmap::DashMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ABTestStatus {
    Draft,
    Running,
    Completed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model_id: String,
    pub version_a: String,
    pub version_b: String,
    pub traffic_split: f64,
    pub status: ABTestStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
}

impl ABTest {
    pub fn new(
        name: String,
        description: Option<String>,
        model_id: String,
        version_a: String,
        version_b: String,
        traffic_split: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            model_id,
            version_a,
            version_b,
            traffic_split,
            status: ABTestStatus::Draft,
            started_at: None,
            ended_at: None,
            created_at: Utc::now(),
            metrics: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub test_id: String,
    pub winner: Option<String>,
    pub version_a_stats: VersionStats,
    pub version_b_stats: VersionStats,
    pub confidence_level: f64,
    pub is_statistically_significant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionStats {
    pub impressions: u64,
    pub conversions: u64,
    pub conversion_rate: f64,
    pub avg_latency_ms: f64,
}

pub struct ABTestManager {
    tests: DashMap<String, ABTest>,
    test_results: DashMap<String, ABTestResult>,
    model_tests: DashMap<String, Vec<String>>,
    active_test: DashMap<String, String>,
}

impl ABTestManager {
    pub fn new() -> Self {
        Self {
            tests: DashMap::new(),
            test_results: DashMap::new(),
            model_tests: DashMap::new(),
            active_test: DashMap::new(),
        }
    }

    pub fn create_test(&self, test: ABTest) -> String {
        let test_id = test.id.clone();
        let model_id = test.model_id.clone();

        self.model_tests
            .entry(model_id)
            .or_default()
            .push(test_id.clone());

        self.tests.insert(test_id.clone(), test);

        test_id
    }

    pub fn get_test(&self, test_id: &str) -> Option<ABTest> {
        self.tests.get(test_id).map(|t| t.clone())
    }

    pub fn start_test(&self, test_id: &str) -> bool {
        if let Some(mut test) = self.tests.get_mut(test_id) {
            test.status = ABTestStatus::Running;
            test.started_at = Some(Utc::now());
            self.active_test
                .insert(test.model_id.clone(), test_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn stop_test(&self, test_id: &str) -> bool {
        if let Some(mut test) = self.tests.get_mut(test_id) {
            test.status = ABTestStatus::Completed;
            test.ended_at = Some(Utc::now());
            self.active_test.remove(&test.model_id);
            true
        } else {
            false
        }
    }

    pub fn pause_test(&self, test_id: &str) -> bool {
        if let Some(mut test) = self.tests.get_mut(test_id) {
            test.status = ABTestStatus::Paused;
            true
        } else {
            false
        }
    }

    pub fn list_tests(&self, model_id: &str) -> Vec<ABTest> {
        self.model_tests
            .get(model_id)
            .map(|ids| {
                let mut tests: Vec<ABTest> =
                    ids.iter().filter_map(|id| self.get_test(id)).collect();

                tests.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                tests
            })
            .unwrap_or_default()
    }

    pub fn get_active_test(&self, model_id: &str) -> Option<ABTest> {
        self.active_test
            .get(model_id)
            .and_then(|id| self.get_test(&id))
    }

    pub fn select_version(&self, test_id: &str, user_id: Option<&str>) -> Option<String> {
        let test = self.get_test(test_id)?;

        if test.status != ABTestStatus::Running {
            return Some(test.version_a.clone());
        }

        let hash = user_id
            .map(|id| {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                id.hash(&mut hasher);
                hasher.finish() as f64 / u64::MAX as f64
            })
            .unwrap_or_else(rand::random::<f64>);

        if hash < test.traffic_split {
            Some(test.version_a.clone())
        } else {
            Some(test.version_b.clone())
        }
    }

    pub fn record_conversion(
        &self,
        test_id: &str,
        version: &str,
        latency_ms: f64,
        converted: bool,
    ) {
        if let Some(mut result) = self.test_results.get_mut(test_id) {
            let stats = if version == result.version_a_stats.impressions.to_string() {
                &mut result.version_a_stats
            } else {
                &mut result.version_b_stats
            };

            stats.impressions += 1;
            stats.avg_latency_ms = (stats.avg_latency_ms * (stats.impressions - 1) as f64
                + latency_ms)
                / stats.impressions as f64;

            if converted {
                stats.conversions += 1;
            }

            stats.conversion_rate = if stats.impressions > 0 {
                stats.conversions as f64 / stats.impressions as f64
            } else {
                0.0
            };
        }
    }

    pub fn get_test_result(&self, test_id: &str) -> Option<ABTestResult> {
        self.test_results.get(test_id).map(|r| r.clone())
    }

    pub fn compute_test_result(&self, test_id: &str) -> Option<ABTestResult> {
        let _test = self.get_test(test_id)?;

        let result = ABTestResult {
            test_id: test_id.to_string(),
            winner: None,
            version_a_stats: VersionStats::default(),
            version_b_stats: VersionStats::default(),
            confidence_level: 0.95,
            is_statistically_significant: false,
        };

        self.test_results
            .insert(test_id.to_string(), result.clone());
        Some(result)
    }
}

impl Default for ABTestManager {
    fn default() -> Self {
        Self::new()
    }
}
