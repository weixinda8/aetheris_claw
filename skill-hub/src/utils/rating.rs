use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SkillRatingInput {
    pub skill_id: Uuid,
    pub average_rating: f64,
    pub rating_count: i32,
    pub download_count: i64,
    pub success_rate: f64,
    pub execution_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub recent_downloads: i64,
    pub recent_executions: i64,
    pub audit_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRating {
    pub skill_id: Uuid,
    pub bayesian_rating: f64,
    pub normalized_downloads: f64,
    pub smoothed_success_rate: f64,
    pub activity_score: f64,
    pub overall_score: f64,
    pub audit_quality_score: f64,
    pub trending_score: f64,
}

pub struct RatingConfig {
    pub min_ratings_for_bayesian: i32,
    pub global_average_rating: f64,
    pub max_downloads_for_normalization: i64,
    pub laplace_smoothing_alpha: f64,
    pub laplace_smoothing_beta: f64,
    pub activity_decay_days: f64,
    pub rating_weight: f64,
    pub download_weight: f64,
    pub success_weight: f64,
    pub audit_weight: f64,
    pub activity_weight: f64,
    pub max_ratings_per_user: i32,
    pub max_downloads_per_ip: i32,
    pub min_executions_for_success: i64,
}

impl Default for RatingConfig {
    fn default() -> Self {
        Self {
            min_ratings_for_bayesian: 10,
            global_average_rating: 3.5,
            max_downloads_for_normalization: 10000,
            laplace_smoothing_alpha: 2.0,
            laplace_smoothing_beta: 3.0,
            activity_decay_days: 30.0,
            rating_weight: 0.35,
            download_weight: 0.20,
            success_weight: 0.25,
            audit_weight: 0.10,
            activity_weight: 0.10,
            max_ratings_per_user: 1,
            max_downloads_per_ip: 5,
            min_executions_for_success: 10,
        }
    }
}

pub fn calculate_bayesian_average(
    average_rating: f64,
    rating_count: i32,
    config: &RatingConfig,
) -> f64 {
    let n = rating_count as f64;
    let m = config.min_ratings_for_bayesian as f64;
    let c = config.global_average_rating;
    let r = average_rating;

    (c * m + r * n) / (m + n)
}

pub fn normalize_downloads(download_count: i64, config: &RatingConfig) -> f64 {
    let x = download_count as f64;
    let max_dl = config.max_downloads_for_normalization as f64;

    (1.0 + x.ln()).min(1.0 + max_dl.ln()) / (1.0 + max_dl.ln())
}

pub fn smooth_success_rate(
    success_rate: f64,
    execution_count: i64,
    config: &RatingConfig,
) -> f64 {
    let alpha = config.laplace_smoothing_alpha;
    let beta = config.laplace_smoothing_beta;

    let successes = success_rate * execution_count as f64;
    let failures = execution_count as f64 - successes;

    (successes + alpha) / (execution_count as f64 + alpha + beta)
}

pub fn calculate_activity_score(
    updated_at: DateTime<Utc>,
    recent_downloads: i64,
    recent_executions: i64,
    config: &RatingConfig,
) -> f64 {
    let now = Utc::now();
    let days_since_update = (now - updated_at).num_days() as f64;

    let time_decay = (-days_since_update / config.activity_decay_days).exp();

    let download_activity = (recent_downloads as f64).min(100.0) / 100.0;
    let execution_activity = (recent_executions as f64).min(100.0) / 100.0;

    time_decay * (0.5 * download_activity + 0.5 * execution_activity)
}

pub fn calculate_trending_score(
    recent_downloads: i64,
    recent_executions: i64,
    rating_count: i32,
    published_at: Option<DateTime<Utc>>,
    config: &RatingConfig,
) -> f64 {
    let now = Utc::now();

    let recency_factor = if let Some(published) = published_at {
        let days_since_published = (now - published).num_days() as f64;
        (-days_since_published / 7.0).exp()
    } else {
        0.5
    };

    let download_trend = (recent_downloads as f64).min(100.0) / 100.0;
    let execution_trend = (recent_executions as f64).min(100.0) / 100.0;
    let rating_trend = (rating_count as f64).min(50.0) / 50.0;

    recency_factor * (0.4 * download_trend + 0.3 * execution_trend + 0.3 * rating_trend)
}

pub fn calculate_overall_score(
    bayesian_rating: f64,
    normalized_downloads: f64,
    smoothed_success_rate: f64,
    activity_score: f64,
    audit_quality_score: f64,
    config: &RatingConfig,
) -> f64 {
    let normalized_rating = bayesian_rating / 5.0;

    config.rating_weight * normalized_rating
        + config.download_weight * normalized_downloads
        + config.success_weight * smoothed_success_rate
        + config.audit_weight * audit_quality_score
        + config.activity_weight * activity_score
}

pub fn calculate_skill_rating(
    input: &SkillRatingInput,
    config: &RatingConfig,
) -> SkillRating {
    let bayesian_rating = calculate_bayesian_average(input.average_rating, input.rating_count, config);
    let normalized_downloads = normalize_downloads(input.download_count, config);
    let smoothed_success_rate = smooth_success_rate(input.success_rate, input.execution_count, config);
    let activity_score = calculate_activity_score(
        input.updated_at,
        input.recent_downloads,
        input.recent_executions,
        config,
    );
    let trending_score = calculate_trending_score(
        input.recent_downloads,
        input.recent_executions,
        input.rating_count,
        input.published_at,
        config,
    );
    let overall_score = calculate_overall_score(
        bayesian_rating,
        normalized_downloads,
        smoothed_success_rate,
        activity_score,
        input.audit_quality_score,
        config,
    );

    SkillRating {
        skill_id: input.skill_id,
        bayesian_rating,
        normalized_downloads,
        smoothed_success_rate,
        activity_score,
        overall_score,
        audit_quality_score: input.audit_quality_score,
        trending_score,
    }
}

pub fn validate_rating(
    user_id: Uuid,
    skill_id: Uuid,
    existing_ratings: &[(Uuid, Uuid)],
    config: &RatingConfig,
) -> bool {
    let user_ratings = existing_ratings
        .iter()
        .filter(|(uid, sid)| *uid == user_id && *sid == skill_id)
        .count();

    user_ratings < config.max_ratings_per_user as usize
}

pub fn validate_download(
    ip_address: Option<&str>,
    skill_id: Uuid,
    recent_downloads: &[(Option<String>, Uuid, DateTime<Utc>)],
    config: &RatingConfig,
) -> bool {
    if let Some(ip) = ip_address {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);

        let ip_downloads = recent_downloads
            .iter()
            .filter(|(download_ip, sid, time)| {
                *download_ip == Some(ip.to_string())
                    && *sid == skill_id
                    && *time >= one_day_ago
            })
            .count();

        return ip_downloads < config.max_downloads_per_ip as usize;
    }
    true
}

pub fn validate_success_rate(
    success_count: i64,
    total_count: i64,
    config: &RatingConfig,
) -> f64 {
    if total_count < config.min_executions_for_success {
        0.5
    } else {
        success_count as f64 / total_count as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortBy {
    Recommended,
    Newest,
    MostDownloaded,
    HighestRated,
    MostActive,
    Trending,
}

impl SortBy {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "recommended" => Some(SortBy::Recommended),
            "newest" => Some(SortBy::Newest),
            "most_downloaded" | "mostdownloaded" => Some(SortBy::MostDownloaded),
            "highest_rated" | "highestrated" => Some(SortBy::HighestRated),
            "most_active" | "mostactive" => Some(SortBy::MostActive),
            "trending" | "trending" => Some(SortBy::Trending),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortBy::Recommended => "recommended",
            SortBy::Newest => "newest",
            SortBy::MostDownloaded => "most_downloaded",
            SortBy::HighestRated => "highest_rated",
            SortBy::MostActive => "most_active",
            SortBy::Trending => "trending",
        }
    }
}

pub fn sort_skills_with_ratings(
    mut skills: Vec<(SkillRatingInput, SkillRating)>,
    sort_by: &SortBy,
) -> Vec<(SkillRatingInput, SkillRating)> {
    match sort_by {
        SortBy::Recommended => {
            skills.sort_by(|a, b| b.1.overall_score.partial_cmp(&a.1.overall_score).unwrap());
        }
        SortBy::Newest => {
            skills.sort_by(|a, b| b.0.published_at.cmp(&a.0.published_at));
        }
        SortBy::MostDownloaded => {
            skills.sort_by(|a, b| b.0.download_count.cmp(&a.0.download_count));
        }
        SortBy::HighestRated => {
            skills.sort_by(|a, b| b.1.bayesian_rating.partial_cmp(&a.1.bayesian_rating).unwrap());
        }
        SortBy::MostActive => {
            skills.sort_by(|a, b| b.1.activity_score.partial_cmp(&a.1.activity_score).unwrap());
        }
        SortBy::Trending => {
            skills.sort_by(|a, b| b.1.trending_score.partial_cmp(&a.1.trending_score).unwrap());
        }
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_bayesian_average() {
        let config = RatingConfig::default();
        
        let result = calculate_bayesian_average(5.0, 5, &config);
        assert!(result < 5.0);
        
        let result = calculate_bayesian_average(5.0, 100, &config);
        assert!(result > 4.5);
    }

    #[test]
    fn test_normalize_downloads() {
        let config = RatingConfig::default();
        
        let result = normalize_downloads(0, &config);
        assert_eq!(result, 0.0);
        
        let result = normalize_downloads(100, &config);
        assert!(result > 0.0 && result < 1.0);
        
        let result = normalize_downloads(10000, &config);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_smooth_success_rate() {
        let config = RatingConfig::default();
        
        let result = smooth_success_rate(1.0, 5, &config);
        assert!(result < 1.0);
        
        let result = smooth_success_rate(1.0, 100, &config);
        assert!(result > 0.9);
    }

    #[test]
    fn test_sort_by() {
        assert_eq!(SortBy::from_str("recommended"), Some(SortBy::Recommended));
        assert_eq!(SortBy::from_str("newest"), Some(SortBy::Newest));
        assert_eq!(SortBy::from_str("most_downloaded"), Some(SortBy::MostDownloaded));
        assert_eq!(SortBy::from_str("invalid"), None);
    }
}
