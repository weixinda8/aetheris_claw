use crate::soul::Soul;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
    pub build: Option<String>,
}

impl PersonaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
            build: None,
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, '-').collect();
        let main_part = parts[0];
        let pre_build = parts.get(1).copied().unwrap_or("");

        let mut build_part = None;
        let pre_build_parts: Vec<&str> = pre_build.splitn(2, '+').collect();
        let pre_part = if pre_build_parts.len() == 2 {
            build_part = Some(pre_build_parts[1].to_string());
            pre_build_parts[0]
        } else {
            pre_build
        };

        let version_parts: Vec<&str> = main_part.split('.').collect();
        if version_parts.len() != 3 {
            return Err(AetherisError::Validation(
                "Version must be in format major.minor.patch".to_string(),
            ));
        }

        let major = version_parts[0]
            .parse()
            .map_err(|_| AetherisError::Validation("Invalid major version".to_string()))?;
        let minor = version_parts[1]
            .parse()
            .map_err(|_| AetherisError::Validation("Invalid minor version".to_string()))?;
        let patch = version_parts[2]
            .parse()
            .map_err(|_| AetherisError::Validation("Invalid patch version".to_string()))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre: if pre_part.is_empty() {
                None
            } else {
                Some(pre_part.to_string())
            },
            build: build_part,
        })
    }

    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    pub fn increment_patch(&mut self) {
        self.patch += 1;
    }
}

impl fmt::Display for PersonaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for PersonaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PersonaVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match (&self.pre, &other.pre) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(pre1), Some(pre2)) => pre1.cmp(pre2),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaMetadata {
    pub persona_id: String,
    pub name: String,
    pub description: String,
    pub version: PersonaVersion,
    pub author: String,
    pub author_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_official: bool,
    pub is_published: bool,
    pub is_verified: bool,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub screenshot_urls: Vec<String>,
    pub compatibility: Vec<String>,
}

impl PersonaMetadata {
    pub fn new(name: String, description: String, author: String, author_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            persona_id: Uuid::new_v4().to_string(),
            name,
            description,
            version: PersonaVersion::new(1, 0, 0),
            author,
            author_id,
            created_at: now,
            updated_at: now,
            is_official: false,
            is_published: false,
            is_verified: false,
            tags: Vec::new(),
            categories: Vec::new(),
            license: None,
            homepage: None,
            repository: None,
            screenshot_urls: Vec::new(),
            compatibility: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AetherisError::Validation(
                "Persona name cannot be empty".to_string(),
            ));
        }
        if self.description.is_empty() {
            return Err(AetherisError::Validation(
                "Persona description cannot be empty".to_string(),
            ));
        }
        if self.author.is_empty() {
            return Err(AetherisError::Validation(
                "Persona author cannot be empty".to_string(),
            ));
        }
        if self.author_id.is_empty() {
            return Err(AetherisError::Validation(
                "Persona author_id cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRating {
    pub rating_id: String,
    pub persona_id: String,
    pub user_id: String,
    pub rating: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PersonaRating {
    pub fn new(persona_id: String, user_id: String, rating: u8) -> Result<Self> {
        if !(1..=5).contains(&rating) {
            return Err(AetherisError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }
        let now = chrono::Utc::now();
        Ok(Self {
            rating_id: Uuid::new_v4().to_string(),
            persona_id,
            user_id,
            rating,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_rating(&mut self, new_rating: u8) -> Result<()> {
        if !(1..=5).contains(&new_rating) {
            return Err(AetherisError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }
        self.rating = new_rating;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaReview {
    pub review_id: String,
    pub persona_id: String,
    pub user_id: String,
    pub user_name: String,
    pub rating: PersonaRating,
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
    pub not_helpful_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_edited: bool,
}

impl PersonaReview {
    pub fn new(
        persona_id: String,
        user_id: String,
        user_name: String,
        rating: u8,
        title: String,
        content: String,
    ) -> Result<Self> {
        let persona_rating = PersonaRating::new(persona_id.clone(), user_id.clone(), rating)?;
        let now = chrono::Utc::now();
        Ok(Self {
            review_id: Uuid::new_v4().to_string(),
            persona_id,
            user_id,
            user_name,
            rating: persona_rating,
            title,
            content,
            helpful_count: 0,
            not_helpful_count: 0,
            created_at: now,
            updated_at: now,
            is_edited: false,
        })
    }

    pub fn update(&mut self, title: String, content: String, rating: u8) -> Result<()> {
        self.title = title;
        self.content = content;
        self.rating.update_rating(rating)?;
        self.updated_at = chrono::Utc::now();
        self.is_edited = true;
        Ok(())
    }

    pub fn mark_helpful(&mut self) {
        self.helpful_count += 1;
    }

    pub fn mark_not_helpful(&mut self) {
        self.not_helpful_count += 1;
    }

    pub fn helpful_ratio(&self) -> f32 {
        let total = self.helpful_count + self.not_helpful_count;
        if total == 0 {
            0.0
        } else {
            self.helpful_count as f32 / total as f32
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaEntry {
    pub metadata: PersonaMetadata,
    pub soul: Soul,
    pub ratings: Vec<PersonaRating>,
    pub reviews: Vec<PersonaReview>,
    pub download_count: u64,
    pub view_count: u64,
    pub price: Option<f64>,
    pub currency: Option<String>,
}

impl PersonaEntry {
    pub fn new(metadata: PersonaMetadata, soul: Soul) -> Self {
        Self {
            metadata,
            soul,
            ratings: Vec::new(),
            reviews: Vec::new(),
            download_count: 0,
            view_count: 0,
            price: None,
            currency: None,
        }
    }

    pub fn average_rating(&self) -> f32 {
        if self.ratings.is_empty() {
            return 0.0;
        }
        let total: u32 = self.ratings.iter().map(|r| r.rating as u32).sum();
        total as f32 / self.ratings.len() as f32
    }

    pub fn rating_count(&self) -> u32 {
        self.ratings.len() as u32
    }

    pub fn review_count(&self) -> u32 {
        self.reviews.len() as u32
    }

    pub fn increment_download(&mut self) {
        self.download_count += 1;
    }

    pub fn increment_view(&mut self) {
        self.view_count += 1;
    }
}

#[derive(Debug, Clone)]
pub struct PersonaMarketplace {
    personas: Arc<DashMap<String, PersonaEntry>>,
    persona_name_index: Arc<DashMap<String, Vec<String>>>,
    author_index: Arc<DashMap<String, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    category_index: Arc<DashMap<String, Vec<String>>>,
    user_ratings: Arc<DashMap<String, Vec<PersonaRating>>>,
    user_reviews: Arc<DashMap<String, Vec<PersonaReview>>>,
}

impl PersonaMarketplace {
    pub fn new() -> Self {
        Self {
            personas: Arc::new(DashMap::new()),
            persona_name_index: Arc::new(DashMap::new()),
            author_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            category_index: Arc::new(DashMap::new()),
            user_ratings: Arc::new(DashMap::new()),
            user_reviews: Arc::new(DashMap::new()),
        }
    }

    pub fn register_persona(&self, persona: PersonaEntry) -> Result<String> {
        persona.metadata.validate()?;

        let persona_id = persona.metadata.persona_id.clone();

        if self.personas.contains_key(&persona_id) {
            return Err(AetherisError::Validation(format!(
                "Persona with ID '{}' already exists",
                persona_id
            )));
        }

        info!(
            "Registering persona: {} (v{}) by {}",
            persona.metadata.name, persona.metadata.version, persona.metadata.author
        );

        self.update_indices(&persona);
        self.personas.insert(persona_id.clone(), persona);

        Ok(persona_id)
    }

    fn update_indices(&self, persona: &PersonaEntry) {
        let persona_id = persona.metadata.persona_id.clone();

        let name_key = persona.metadata.name.to_lowercase();
        self.persona_name_index
            .entry(name_key)
            .or_default()
            .push(persona_id.clone());

        self.author_index
            .entry(persona.metadata.author_id.clone())
            .or_default()
            .push(persona_id.clone());

        for tag in &persona.metadata.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(persona_id.clone());
        }

        for category in &persona.metadata.categories {
            self.category_index
                .entry(category.clone())
                .or_default()
                .push(persona_id.clone());
        }
    }

    fn remove_from_indices(&self, persona_id: &str, persona: &PersonaEntry) {
        let name_key = persona.metadata.name.to_lowercase();
        if let Some(mut ids) = self.persona_name_index.get_mut(&name_key) {
            ids.retain(|id| id != persona_id);
            if ids.is_empty() {
                self.persona_name_index.remove(&name_key);
            }
        }

        if let Some(mut ids) = self.author_index.get_mut(&persona.metadata.author_id) {
            ids.retain(|id| id != persona_id);
            if ids.is_empty() {
                self.author_index.remove(&persona.metadata.author_id);
            }
        }

        for tag in &persona.metadata.tags {
            if let Some(mut ids) = self.tag_index.get_mut(tag) {
                ids.retain(|id| id != persona_id);
                if ids.is_empty() {
                    self.tag_index.remove(tag);
                }
            }
        }

        for category in &persona.metadata.categories {
            if let Some(mut ids) = self.category_index.get_mut(category) {
                ids.retain(|id| id != persona_id);
                if ids.is_empty() {
                    self.category_index.remove(category);
                }
            }
        }
    }

    pub fn get_persona(&self, persona_id: &str) -> Option<PersonaEntry> {
        self.personas
            .get(persona_id)
            .map(|entry| entry.value().clone())
    }

    pub fn get_persona_mut(
        &self,
        persona_id: &str,
    ) -> Option<dashmap::mapref::one::RefMut<'_, String, PersonaEntry>> {
        self.personas.get_mut(persona_id)
    }

    pub fn list_personas(&self) -> Vec<PersonaEntry> {
        self.personas
            .iter()
            .filter(|entry| entry.value().metadata.is_published)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn list_all_personas(&self) -> Vec<PersonaEntry> {
        self.personas
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn update_persona(&self, persona_id: &str, metadata: PersonaMetadata) -> Result<()> {
        metadata.validate()?;

        let mut entry = self
            .personas
            .get_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        let old_metadata = entry.metadata.clone();
        self.remove_from_indices(persona_id, &entry);

        entry.metadata = metadata;
        entry.metadata.updated_at = chrono::Utc::now();

        self.update_indices(&entry);

        info!(
            "Updated persona: {} from v{} to v{}",
            entry.metadata.name, old_metadata.version, entry.metadata.version
        );

        Ok(())
    }

    pub fn delete_persona(&self, persona_id: &str) -> Result<()> {
        let (_, persona) = self
            .personas
            .remove(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        self.remove_from_indices(persona_id, &persona);

        info!("Deleted persona: {}", persona.metadata.name);

        Ok(())
    }

    pub fn rate_persona(&self, persona_id: &str, user_id: &str, rating: u8) -> Result<()> {
        let mut persona = self
            .get_persona_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        if let Some(existing_rating) = persona.ratings.iter_mut().find(|r| r.user_id == user_id) {
            existing_rating.update_rating(rating)?;
        } else {
            let persona_rating =
                PersonaRating::new(persona_id.to_string(), user_id.to_string(), rating)?;
            persona.ratings.push(persona_rating.clone());

            self.user_ratings
                .entry(user_id.to_string())
                .or_default()
                .push(persona_rating);
        }

        info!(
            "Rated persona {} by user {}: {}",
            persona_id, user_id, rating
        );

        Ok(())
    }

    pub fn add_review(&self, review: PersonaReview) -> Result<String> {
        let mut persona = self.get_persona_mut(&review.persona_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Persona not found: {}", review.persona_id))
        })?;

        if let Some(existing_review) = persona.reviews.iter().find(|r| r.user_id == review.user_id)
        {
            return Err(AetherisError::Validation(format!(
                "User already reviewed this persona (review ID: {})",
                existing_review.review_id
            )));
        }

        if let Some(existing_rating) = persona
            .ratings
            .iter_mut()
            .find(|r| r.user_id == review.user_id)
        {
            existing_rating.update_rating(review.rating.rating)?;
        } else {
            let persona_rating = PersonaRating::new(
                review.persona_id.clone(),
                review.user_id.clone(),
                review.rating.rating,
            )?;
            persona.ratings.push(persona_rating);
        }

        let review_id = review.review_id.clone();
        let persona_id = review.persona_id.clone();
        let user_id = review.user_id.clone();

        persona.reviews.push(review.clone());
        self.user_reviews
            .entry(user_id.clone())
            .or_default()
            .push(review);

        info!(
            "Added review for persona {} by user {}: {}",
            persona_id, user_id, review_id
        );

        Ok(review_id)
    }

    pub fn get_reviews(&self, persona_id: &str) -> Vec<PersonaReview> {
        self.personas
            .get(persona_id)
            .map(|entry| entry.value().reviews.clone())
            .unwrap_or_default()
    }

    pub fn download_persona(&self, persona_id: &str) -> Result<Soul> {
        let mut persona = self
            .get_persona_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        persona.increment_download();

        info!("Downloaded persona: {}", persona_id);

        Ok(persona.soul.clone())
    }

    pub fn view_persona(&self, persona_id: &str) -> Result<PersonaEntry> {
        let mut persona = self
            .get_persona_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        persona.increment_view();

        Ok(persona.value().clone())
    }

    pub fn search_personas(&self, query: &str) -> Vec<PersonaEntry> {
        let query_lower = query.to_lowercase();
        self.personas
            .iter()
            .filter(|entry| {
                let persona = entry.value();
                if !persona.metadata.is_published {
                    return false;
                }

                persona.metadata.name.to_lowercase().contains(&query_lower)
                    || persona
                        .metadata
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
                    || persona
                        .metadata
                        .author
                        .to_lowercase()
                        .contains(&query_lower)
                    || persona
                        .metadata
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || persona
                        .metadata
                        .categories
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_personas_by_author(&self, author_id: &str) -> Vec<PersonaEntry> {
        self.author_index
            .get(author_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_persona(id))
                    .filter(|p| p.metadata.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_personas_by_tag(&self, tag: &str) -> Vec<PersonaEntry> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_persona(id))
                    .filter(|p| p.metadata.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_personas_by_category(&self, category: &str) -> Vec<PersonaEntry> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_persona(id))
                    .filter(|p| p.metadata.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_popular_personas(&self, limit: usize) -> Vec<PersonaEntry> {
        let mut personas = self.list_personas();
        personas.sort_by(|a, b| {
            b.download_count.cmp(&a.download_count).then_with(|| {
                b.average_rating()
                    .partial_cmp(&a.average_rating())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        personas.truncate(limit);
        personas
    }

    pub fn get_top_rated_personas(&self, limit: usize) -> Vec<PersonaEntry> {
        let mut personas = self.list_personas();
        personas.sort_by(|a, b| {
            b.average_rating()
                .partial_cmp(&a.average_rating())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.rating_count().cmp(&a.rating_count()))
        });
        personas.truncate(limit);
        personas
    }

    pub fn persona_count(&self) -> usize {
        self.personas.len()
    }

    pub fn published_persona_count(&self) -> usize {
        self.list_personas().len()
    }

    pub fn get_user_ratings(&self, user_id: &str) -> Vec<PersonaRating> {
        self.user_ratings
            .get(user_id)
            .map(|ratings| ratings.value().clone())
            .unwrap_or_default()
    }

    pub fn get_user_reviews(&self, user_id: &str) -> Vec<PersonaReview> {
        self.user_reviews
            .get(user_id)
            .map(|reviews| reviews.value().clone())
            .unwrap_or_default()
    }

    pub fn publish_persona(&self, persona_id: &str) -> Result<()> {
        let mut persona = self
            .get_persona_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        persona.metadata.is_published = true;
        persona.metadata.updated_at = chrono::Utc::now();

        info!("Published persona: {}", persona_id);

        Ok(())
    }

    pub fn unpublish_persona(&self, persona_id: &str) -> Result<()> {
        let mut persona = self
            .get_persona_mut(persona_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Persona not found: {}", persona_id)))?;

        persona.metadata.is_published = false;
        persona.metadata.updated_at = chrono::Utc::now();

        info!("Unpublished persona: {}", persona_id);

        Ok(())
    }
}

impl Default for PersonaMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_persona_version() {
        let version = PersonaVersion::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let parsed = PersonaVersion::parse("1.2.3").unwrap();
        assert_eq!(parsed, version);

        let parsed_with_pre = PersonaVersion::parse("1.2.3-beta").unwrap();
        assert_eq!(parsed_with_pre.pre, Some("beta".to_string()));

        let parsed_with_build = PersonaVersion::parse("1.2.3+build123").unwrap();
        assert_eq!(parsed_with_build.build, Some("build123".to_string()));
    }

    #[test]
    fn test_persona_version_comparison() {
        let v1_0_0 = PersonaVersion::new(1, 0, 0);
        let v1_1_0 = PersonaVersion::new(1, 1, 0);
        let v2_0_0 = PersonaVersion::new(2, 0, 0);
        let v1_0_0_beta = PersonaVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre: Some("beta".to_string()),
            build: None,
        };

        assert!(v1_0_0 < v1_1_0);
        assert!(v1_1_0 < v2_0_0);
        assert!(v1_0_0_beta < v1_0_0);
    }

    #[test]
    fn test_persona_version_increment() {
        let mut version = PersonaVersion::new(1, 2, 3);
        version.increment_patch();
        assert_eq!(version.to_string(), "1.2.4");

        let mut version = PersonaVersion::new(1, 2, 3);
        version.increment_minor();
        assert_eq!(version.to_string(), "1.3.0");

        let mut version = PersonaVersion::new(1, 2, 3);
        version.increment_major();
        assert_eq!(version.to_string(), "2.0.0");
    }

    #[test]
    fn test_persona_metadata_validation() {
        let valid = PersonaMetadata::new(
            "Test Persona".to_string(),
            "Test Description".to_string(),
            "Test Author".to_string(),
            "test-author-id".to_string(),
        );
        assert!(valid.validate().is_ok());

        let mut invalid_name = valid.clone();
        invalid_name.name = String::new();
        assert!(invalid_name.validate().is_err());

        let mut invalid_description = valid.clone();
        invalid_description.description = String::new();
        assert!(invalid_description.validate().is_err());
    }

    #[test]
    fn test_persona_rating() {
        let rating = PersonaRating::new("persona-1".to_string(), "user-1".to_string(), 5).unwrap();
        assert_eq!(rating.rating, 5);

        let invalid_rating = PersonaRating::new("persona-1".to_string(), "user-1".to_string(), 6);
        assert!(invalid_rating.is_err());

        let invalid_rating_low =
            PersonaRating::new("persona-1".to_string(), "user-1".to_string(), 0);
        assert!(invalid_rating_low.is_err());
    }

    #[test]
    fn test_persona_review() {
        let review = PersonaReview::new(
            "persona-1".to_string(),
            "user-1".to_string(),
            "User One".to_string(),
            4,
            "Great persona!".to_string(),
            "This is a really helpful persona.".to_string(),
        )
        .unwrap();

        assert_eq!(review.title, "Great persona!");
        assert_eq!(review.rating.rating, 4);
    }

    #[test]
    fn test_marketplace_basic() {
        let marketplace = PersonaMarketplace::new();
        assert_eq!(marketplace.persona_count(), 0);
        assert_eq!(marketplace.published_persona_count(), 0);
    }

    #[test]
    fn test_register_persona() {
        let marketplace = PersonaMarketplace::new();
        let temp_dir = tempdir().unwrap();
        let souls_dir = temp_dir.path().join("souls");
        std::fs::create_dir_all(&souls_dir).unwrap();

        let soul_content = r#"---
name: Test Soul
description: Test soul description
personality: Friendly
---
Test soul content.
"#;
        let soul_path = souls_dir.join("test-soul.md");
        std::fs::write(&soul_path, soul_content).unwrap();
        let soul = Soul::from_path(soul_path).unwrap();

        let metadata = PersonaMetadata::new(
            "Test Persona".to_string(),
            "Test persona description".to_string(),
            "Test Author".to_string(),
            "test-author-id".to_string(),
        );

        let entry = PersonaEntry::new(metadata, soul);
        let persona_id = marketplace.register_persona(entry).unwrap();

        assert_eq!(marketplace.persona_count(), 1);
        assert_eq!(marketplace.published_persona_count(), 0);

        marketplace.publish_persona(&persona_id).unwrap();
        assert_eq!(marketplace.published_persona_count(), 1);
    }

    #[test]
    fn test_rate_persona() {
        let marketplace = PersonaMarketplace::new();
        let temp_dir = tempdir().unwrap();
        let souls_dir = temp_dir.path().join("souls");
        std::fs::create_dir_all(&souls_dir).unwrap();

        let soul_content = r#"---
name: Test Soul
description: Test soul description
personality: Friendly
---
Test soul content.
"#;
        let soul_path = souls_dir.join("test-soul.md");
        std::fs::write(&soul_path, soul_content).unwrap();
        let soul = Soul::from_path(soul_path).unwrap();

        let metadata = PersonaMetadata::new(
            "Test Persona".to_string(),
            "Test persona description".to_string(),
            "Test Author".to_string(),
            "test-author-id".to_string(),
        );

        let entry = PersonaEntry::new(metadata, soul);
        let persona_id = marketplace.register_persona(entry).unwrap();

        marketplace.rate_persona(&persona_id, "user-1", 5).unwrap();

        let persona = marketplace.get_persona(&persona_id).unwrap();
        assert_eq!(persona.rating_count(), 1);
        assert_eq!(persona.average_rating(), 5.0);
    }

    #[test]
    fn test_add_review() {
        let marketplace = PersonaMarketplace::new();
        let temp_dir = tempdir().unwrap();
        let souls_dir = temp_dir.path().join("souls");
        std::fs::create_dir_all(&souls_dir).unwrap();

        let soul_content = r#"---
name: Test Soul
description: Test soul description
personality: Friendly
---
Test soul content.
"#;
        let soul_path = souls_dir.join("test-soul.md");
        std::fs::write(&soul_path, soul_content).unwrap();
        let soul = Soul::from_path(soul_path).unwrap();

        let metadata = PersonaMetadata::new(
            "Test Persona".to_string(),
            "Test persona description".to_string(),
            "Test Author".to_string(),
            "test-author-id".to_string(),
        );

        let entry = PersonaEntry::new(metadata, soul);
        let persona_id = marketplace.register_persona(entry).unwrap();

        let review = PersonaReview::new(
            persona_id.clone(),
            "user-1".to_string(),
            "User One".to_string(),
            5,
            "Excellent!".to_string(),
            "This is a great persona.".to_string(),
        )
        .unwrap();

        marketplace.add_review(review).unwrap();

        let persona = marketplace.get_persona(&persona_id).unwrap();
        assert_eq!(persona.review_count(), 1);
        assert_eq!(persona.average_rating(), 5.0);
    }

    #[test]
    fn test_search_personas() {
        let marketplace = PersonaMarketplace::new();
        let temp_dir = tempdir().unwrap();
        let souls_dir = temp_dir.path().join("souls");
        std::fs::create_dir_all(&souls_dir).unwrap();

        let soul_content = r#"---
name: Test Soul
description: Test soul description
personality: Friendly
---
Test soul content.
"#;
        let soul_path = souls_dir.join("test-soul.md");
        std::fs::write(&soul_path, soul_content).unwrap();
        let soul = Soul::from_path(soul_path).unwrap();

        let mut metadata = PersonaMetadata::new(
            "Assistant Persona".to_string(),
            "A helpful assistant persona".to_string(),
            "Test Author".to_string(),
            "test-author-id".to_string(),
        );
        metadata.tags = vec!["assistant".to_string(), "helpful".to_string()];
        metadata.categories = vec!["Productivity".to_string()];
        metadata.is_published = true;

        let entry = PersonaEntry::new(metadata, soul.clone());
        let persona_id = marketplace.register_persona(entry).unwrap();

        let results = marketplace.search_personas("assistant");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.persona_id, persona_id);

        let results = marketplace.get_personas_by_tag("helpful");
        assert_eq!(results.len(), 1);

        let results = marketplace.get_personas_by_category("Productivity");
        assert_eq!(results.len(), 1);
    }
}
