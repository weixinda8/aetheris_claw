use crate::skill::clawhub::ClawHubImporter;
use crate::skill::{
    BaseSkill, CallMode, PermissionLevel, SkillMetadata, SkillPriority, SkillRegistry, Version,
};
use crate::utils::AetherisError;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillListing {
    pub listing_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub author_url: Option<String>,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub is_free: bool,
    pub is_verified: bool,
    pub is_official: bool,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub compatible_versions: Vec<String>,
    pub license: Option<String>,
    pub readme: Option<String>,
    pub changelog: Option<String>,
    pub icon_url: Option<String>,
    pub screenshots: Vec<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub issue_tracker_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReview {
    pub review_id: String,
    pub listing_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub rating: u8,
    pub title: Option<String>,
    pub comment: Option<String>,
    pub helpful_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCollection {
    pub collection_id: String,
    pub name: String,
    pub description: String,
    pub author_id: String,
    pub author_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub skills: Vec<String>,
    pub is_public: bool,
    pub follower_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub author: Option<String>,
    pub min_rating: Option<f32>,
    pub min_downloads: Option<u64>,
    pub is_free: Option<bool>,
    pub is_verified: Option<bool>,
    pub is_official: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub total: u64,
    pub page: u32,
    pub limit: u32,
    pub results: Vec<SkillListing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub license: Option<String>,
    pub readme: Option<String>,
    pub changelog: Option<String>,
    pub icon_url: Option<String>,
    pub screenshots: Vec<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub compatible_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub rating: u8,
    pub title: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: String,
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToCollectionRequest {
    pub listing_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserCollectionRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFavoriteRequest {
    pub listing_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFromRegistryRequest {
    pub skill_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFromClawHubRequest {
    pub skill_id: String,
    pub version: Option<String>,
}

pub struct SkillMarketplaceState {
    listings: Arc<DashMap<String, SkillListing>>,
    reviews: Arc<DashMap<String, Vec<SkillReview>>>,
    collections: Arc<DashMap<String, SkillCollection>>,
    user_collections: Arc<DashMap<String, Vec<String>>>,
    user_downloads: Arc<DashMap<String, HashSet<String>>>,
    skill_registry: Arc<SkillRegistry>,
    clawhub_importer: Arc<ClawHubImporter>,
    storage_path: PathBuf,
}

impl SkillMarketplaceState {
    pub fn new(
        skill_registry: Arc<SkillRegistry>,
        clawhub_importer: Arc<ClawHubImporter>,
        storage_path: PathBuf,
    ) -> std::result::Result<Self, crate::utils::AetherisError> {
        std::fs::create_dir_all(&storage_path)?;

        let state = Self {
            listings: Arc::new(DashMap::new()),
            reviews: Arc::new(DashMap::new()),
            collections: Arc::new(DashMap::new()),
            user_collections: Arc::new(DashMap::new()),
            user_downloads: Arc::new(DashMap::new()),
            skill_registry,
            clawhub_importer,
            storage_path,
        };

        state.load()?;
        Ok(state)
    }

    fn save(&self) -> std::result::Result<(), crate::utils::AetherisError> {
        let listings_path = self.storage_path.join("listings.json");
        let listings: Vec<_> = self.listings.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&listings_path, serde_json::to_string_pretty(&listings)?)?;

        let reviews_path = self.storage_path.join("reviews.json");
        let reviews_map: Vec<(String, Vec<SkillReview>)> = self
            .reviews
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&reviews_path, serde_json::to_string_pretty(&reviews_map)?)?;

        let collections_path = self.storage_path.join("collections.json");
        let collections: Vec<_> = self.collections.iter().map(|e| e.value().clone()).collect();
        std::fs::write(
            &collections_path,
            serde_json::to_string_pretty(&collections)?,
        )?;

        let user_collections_path = self.storage_path.join("user_collections.json");
        let user_collections_map: Vec<(String, Vec<String>)> = self
            .user_collections
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(
            &user_collections_path,
            serde_json::to_string_pretty(&user_collections_map)?,
        )?;

        let user_downloads_path = self.storage_path.join("user_downloads.json");
        let user_downloads_map: Vec<(String, Vec<String>)> = self
            .user_downloads
            .iter()
            .map(|e| (e.key().clone(), e.value().iter().cloned().collect()))
            .collect();
        std::fs::write(
            &user_downloads_path,
            serde_json::to_string_pretty(&user_downloads_map)?,
        )?;

        Ok(())
    }

    fn load(&self) -> std::result::Result<(), crate::utils::AetherisError> {
        let listings_path = self.storage_path.join("listings.json");
        if listings_path.exists() {
            let content = std::fs::read_to_string(&listings_path)?;
            let listings: Vec<SkillListing> = serde_json::from_str(&content)?;
            for listing in listings {
                self.listings.insert(listing.listing_id.clone(), listing);
            }
        }

        let reviews_path = self.storage_path.join("reviews.json");
        if reviews_path.exists() {
            let content = std::fs::read_to_string(&reviews_path)?;
            let reviews_map: Vec<(String, Vec<SkillReview>)> = serde_json::from_str(&content)?;
            for (listing_id, reviews) in reviews_map {
                self.reviews.insert(listing_id, reviews);
            }
        }

        let collections_path = self.storage_path.join("collections.json");
        if collections_path.exists() {
            let content = std::fs::read_to_string(&collections_path)?;
            let collections: Vec<SkillCollection> = serde_json::from_str(&content)?;
            for collection in collections {
                self.collections
                    .insert(collection.collection_id.clone(), collection);
            }
        }

        let user_collections_path = self.storage_path.join("user_collections.json");
        if user_collections_path.exists() {
            let content = std::fs::read_to_string(&user_collections_path)?;
            let user_collections_map: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (user_id, collection_ids) in user_collections_map {
                self.user_collections.insert(user_id, collection_ids);
            }
        }

        let user_downloads_path = self.storage_path.join("user_downloads.json");
        if user_downloads_path.exists() {
            let content = std::fs::read_to_string(&user_downloads_path)?;
            let user_downloads_map: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (user_id, listing_ids) in user_downloads_map {
                self.user_downloads
                    .insert(user_id, listing_ids.into_iter().collect());
            }
        }

        Ok(())
    }

    pub fn get_listing(&self, listing_id: &str) -> Option<SkillListing> {
        self.listings.get(listing_id).map(|l| l.value().clone())
    }

    pub fn create_listing(
        &self,
        user_id: &str,
        request: PublishRequest,
    ) -> std::result::Result<SkillListing, crate::utils::AetherisError> {
        let listing_id = uuid::Uuid::new_v4().to_string();
        let skill_id = format!(
            "{}-{}",
            request.name.to_lowercase().replace(" ", "-"),
            listing_id.split('-').next().unwrap()
        );
        let now = chrono::Utc::now();

        info!(
            "Creating skill listing: skill_id={}, name={}",
            skill_id, request.name
        );

        let listing = SkillListing {
            listing_id: listing_id.clone(),
            skill_id: skill_id.clone(),
            name: request.name.clone(),
            description: request.description.clone(),
            version: request.version.clone(),
            author: user_id.to_string(),
            author_url: None,
            published_at: now,
            updated_at: now,
            categories: request.categories.clone(),
            tags: request.tags.clone(),
            price: request.price,
            currency: request.currency,
            is_free: request.price.is_none() || request.price == Some(0.0),
            is_verified: false,
            is_official: false,
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            compatible_versions: request.compatible_versions,
            license: request.license,
            readme: request.readme,
            changelog: request.changelog,
            icon_url: request.icon_url,
            screenshots: request.screenshots,
            homepage_url: request.homepage_url,
            repository_url: request.repository_url,
            documentation_url: request.documentation_url,
            issue_tracker_url: None,
        };

        self.listings.insert(listing_id.clone(), listing.clone());

        info!("Skill listing created: listing_id={}", listing_id);

        let version = match Version::from_string(&request.version) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "Failed to parse version '{}', using default 1.0.0: {}",
                    request.version, e
                );
                Version::new(1, 0, 0)
            }
        };

        let mut skill_metadata =
            SkillMetadata::new(skill_id.clone(), request.name, version, request.description);
        skill_metadata.tags = request.tags;
        skill_metadata.categories = request.categories;
        skill_metadata.author = Some(user_id.to_string());
        skill_metadata.call_mode = CallMode::Text;
        skill_metadata.permission_level = PermissionLevel::Public;
        skill_metadata.priority = SkillPriority::Medium;

        let skill = Arc::new(BaseSkill::new(skill_metadata));
        self.skill_registry.register(skill);

        info!("Skill registered in SkillRegistry: skill_id={}", skill_id);

        self.save()?;
        Ok(listing)
    }

    pub fn update_listing(
        &self,
        listing_id: &str,
        user_id: &str,
        request: PublishRequest,
    ) -> std::result::Result<SkillListing, crate::utils::AetherisError> {
        info!("Updating skill listing: listing_id={}", listing_id);

        let mut listing = self
            .listings
            .get_mut(listing_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Listing not found: {}", listing_id)))?;

        if listing.author != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to update this listing".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        listing.name = request.name.clone();
        listing.description = request.description.clone();
        listing.version = request.version.clone();
        listing.categories = request.categories.clone();
        listing.tags = request.tags.clone();
        listing.price = request.price;
        listing.currency = request.currency;
        listing.is_free = request.price.is_none() || request.price == Some(0.0);
        listing.compatible_versions = request.compatible_versions;
        listing.license = request.license;
        listing.readme = request.readme;
        listing.changelog = request.changelog;
        listing.icon_url = request.icon_url;
        listing.screenshots = request.screenshots;
        listing.homepage_url = request.homepage_url;
        listing.repository_url = request.repository_url;
        listing.documentation_url = request.documentation_url;
        listing.updated_at = now;

        let updated_listing = listing.value().clone();

        info!("Skill listing updated: listing_id={}", listing_id);

        let version = match Version::from_string(&request.version) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "Failed to parse version '{}', using default 1.0.0: {}",
                    request.version, e
                );
                Version::new(1, 0, 0)
            }
        };

        let mut skill_metadata = SkillMetadata::new(
            listing.skill_id.clone(),
            request.name,
            version,
            request.description,
        );
        skill_metadata.tags = request.tags;
        skill_metadata.categories = request.categories;
        skill_metadata.author = Some(user_id.to_string());
        skill_metadata.updated_at = now;
        skill_metadata.call_mode = CallMode::Text;
        skill_metadata.permission_level = PermissionLevel::Public;
        skill_metadata.priority = SkillPriority::Medium;

        let skill = Arc::new(BaseSkill::new(skill_metadata));
        self.skill_registry.register_with_version(skill, true);

        info!(
            "Skill updated in SkillRegistry: skill_id={}",
            listing.skill_id
        );

        self.save()?;
        Ok(updated_listing)
    }

    pub fn delete_listing(
        &self,
        listing_id: &str,
        user_id: &str,
    ) -> std::result::Result<(), crate::utils::AetherisError> {
        info!("Deleting skill listing: listing_id={}", listing_id);

        let listing = self
            .listings
            .get(listing_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Listing not found: {}", listing_id)))?;

        if listing.author != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to delete this listing".to_string(),
            ));
        }

        let skill_id = listing.skill_id.clone();
        self.listings.remove(listing_id);
        self.skill_registry.unregister(&skill_id)?;

        info!(
            "Skill listing deleted: listing_id={}, skill_id={}",
            listing_id, skill_id
        );

        self.save()?;
        Ok(())
    }

    pub async fn import_from_registry(
        &self,
        skill_id: &str,
    ) -> std::result::Result<SkillListing, crate::utils::AetherisError> {
        info!("Importing skill from SkillRegistry: skill_id={}", skill_id);

        let skill = self.skill_registry.get(skill_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Skill not found in registry: {}", skill_id))
        })?;

        let metadata = skill.metadata();
        let listing_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let listing = SkillListing {
            listing_id: listing_id.clone(),
            skill_id: metadata.id.clone(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            version: metadata.version.to_string(),
            author: metadata
                .author
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            author_url: None,
            published_at: now,
            updated_at: now,
            categories: metadata.categories.clone(),
            tags: metadata.tags.clone(),
            price: None,
            currency: None,
            is_free: true,
            is_verified: false,
            is_official: false,
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            compatible_versions: Vec::new(),
            license: None,
            readme: None,
            changelog: None,
            icon_url: None,
            screenshots: Vec::new(),
            homepage_url: None,
            repository_url: None,
            documentation_url: None,
            issue_tracker_url: None,
        };

        self.listings.insert(listing_id.clone(), listing.clone());

        info!(
            "Skill imported from registry: listing_id={}, skill_id={}",
            listing_id, skill_id
        );

        self.save()?;
        Ok(listing)
    }

    pub async fn import_from_clawhub(
        &self,
        skill_id: &str,
        version: Option<&str>,
    ) -> std::result::Result<SkillListing, crate::utils::AetherisError> {
        info!(
            "Importing skill from ClawHub: skill_id={}, version={:?}",
            skill_id, version
        );

        let skill_info = self.clawhub_importer.get_skill_info(skill_id).await?;

        let _manifest = self
            .clawhub_importer
            .download_skill(skill_id, version)
            .await?;

        let listing_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let listing = SkillListing {
            listing_id: listing_id.clone(),
            skill_id: skill_info.id.clone(),
            name: skill_info.name.clone(),
            description: skill_info.description.clone(),
            version: skill_info.version.clone(),
            author: skill_info.author.clone(),
            author_url: None,
            published_at: now,
            updated_at: now,
            categories: skill_info.categories.clone(),
            tags: skill_info.tags.clone(),
            price: None,
            currency: None,
            is_free: true,
            is_verified: skill_info.downloads > 100,
            is_official: false,
            downloads: skill_info.downloads,
            rating: (skill_info.stars as f32 / 5.0) * 5.0,
            rating_count: skill_info.stars,
            compatible_versions: Vec::new(),
            license: skill_info.license.clone(),
            readme: None,
            changelog: None,
            icon_url: None,
            screenshots: Vec::new(),
            homepage_url: skill_info.homepage.clone(),
            repository_url: skill_info.repository.clone(),
            documentation_url: None,
            issue_tracker_url: None,
        };

        self.listings.insert(listing_id.clone(), listing.clone());

        info!(
            "Skill imported from ClawHub: listing_id={}, skill_id={}",
            listing_id, skill_id
        );

        self.save()?;
        Ok(listing)
    }

    pub fn search_listings(&self, query: SearchQuery) -> SearchResults {
        let mut results: Vec<SkillListing> = self
            .listings
            .iter()
            .filter(|entry| {
                let listing = entry.value();

                if let Some(q) = &query.q {
                    let q_lower = q.to_lowercase();
                    let matches = listing.name.to_lowercase().contains(&q_lower)
                        || listing.description.to_lowercase().contains(&q_lower)
                        || listing
                            .tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(&q_lower))
                        || listing
                            .categories
                            .iter()
                            .any(|c| c.to_lowercase().contains(&q_lower));
                    if !matches {
                        return false;
                    }
                }

                if let Some(category) = &query.category {
                    if !listing.categories.contains(category) {
                        return false;
                    }
                }

                if let Some(tag) = &query.tag {
                    if !listing.tags.contains(tag) {
                        return false;
                    }
                }

                if let Some(author) = &query.author {
                    if listing.author != *author {
                        return false;
                    }
                }

                if let Some(min_rating) = query.min_rating {
                    if listing.rating < min_rating {
                        return false;
                    }
                }

                if let Some(min_downloads) = query.min_downloads {
                    if listing.downloads < min_downloads {
                        return false;
                    }
                }

                if let Some(is_free) = query.is_free {
                    if listing.is_free != is_free {
                        return false;
                    }
                }

                if let Some(is_verified) = query.is_verified {
                    if listing.is_verified != is_verified {
                        return false;
                    }
                }

                if let Some(is_official) = query.is_official {
                    if listing.is_official != is_official {
                        return false;
                    }
                }

                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        let sort_by = query.sort_by.as_deref().unwrap_or("downloads");
        let sort_order = query.sort_order.as_deref().unwrap_or("desc");

        results.sort_by(|a, b| {
            let cmp = match sort_by {
                "downloads" => b.downloads.cmp(&a.downloads),
                "rating" => b
                    .rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal),
                "updated" => b.updated_at.cmp(&a.updated_at),
                "published" => b.published_at.cmp(&a.published_at),
                "name" => a.name.cmp(&b.name),
                _ => b.downloads.cmp(&a.downloads),
            };

            if sort_order == "asc" {
                cmp.reverse()
            } else {
                cmp
            }
        });

        let total = results.len() as u64;
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20);
        let start = ((page - 1) * limit) as usize;
        let end = (start + limit as usize).min(results.len());
        let paginated_results = results.drain(start..end).collect();

        SearchResults {
            total,
            page,
            limit,
            results: paginated_results,
        }
    }

    pub fn download_skill(
        &self,
        listing_id: &str,
        user_id: &str,
    ) -> std::result::Result<SkillListing, crate::utils::AetherisError> {
        let mut listing = self
            .listings
            .get_mut(listing_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Listing not found: {}", listing_id)))?;

        listing.downloads += 1;

        self.user_downloads
            .entry(user_id.to_string())
            .or_default()
            .insert(listing_id.to_string());

        info!("User {} downloaded skill {}", user_id, listing_id);

        self.save()?;
        Ok(listing.value().clone())
    }

    pub fn add_review(
        &self,
        listing_id: &str,
        user_id: &str,
        request: ReviewRequest,
    ) -> std::result::Result<SkillReview, crate::utils::AetherisError> {
        if request.rating < 1 || request.rating > 5 {
            return Err(AetherisError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        let review = SkillReview {
            review_id: uuid::Uuid::new_v4().to_string(),
            listing_id: listing_id.to_string(),
            user_id: user_id.to_string(),
            user_name: None,
            rating: request.rating,
            title: request.title,
            comment: request.comment,
            helpful_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        self.reviews
            .entry(listing_id.to_string())
            .or_default()
            .push(review.clone());

        self.update_listing_rating(listing_id)?;

        info!("User {} added review for {}", user_id, listing_id);

        self.save()?;
        Ok(review)
    }

    pub fn get_reviews(&self, listing_id: &str) -> Vec<SkillReview> {
        self.reviews
            .get(listing_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    fn update_listing_rating(
        &self,
        listing_id: &str,
    ) -> std::result::Result<(), crate::utils::AetherisError> {
        if let Some(reviews) = self.reviews.get(listing_id) {
            let rating_count = reviews.len() as u32;
            if rating_count > 0 {
                let total_rating: u32 = reviews.iter().map(|r| r.rating as u32).sum();
                let average_rating = total_rating as f32 / rating_count as f32;

                if let Some(mut listing) = self.listings.get_mut(listing_id) {
                    listing.rating = average_rating;
                    listing.rating_count = rating_count;
                }
            }
        }

        Ok(())
    }

    pub fn get_popular_listings(&self, limit: usize) -> Vec<SkillListing> {
        let mut listings = self
            .listings
            .iter()
            .map(|entry| entry.value().clone())
            .collect::<Vec<_>>();

        listings.sort_by(|a, b| {
            b.downloads.cmp(&a.downloads).then_with(|| {
                b.rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        listings.truncate(limit);
        listings
    }

    pub fn get_top_rated_listings(&self, limit: usize) -> Vec<SkillListing> {
        let mut listings = self
            .listings
            .iter()
            .map(|entry| entry.value().clone())
            .collect::<Vec<_>>();

        listings.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.rating_count.cmp(&a.rating_count))
        });

        listings.truncate(limit);
        listings
    }

    pub fn get_featured_listings(&self) -> Vec<SkillListing> {
        self.listings
            .iter()
            .filter(|entry| entry.value().is_official || entry.value().is_verified)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn listing_count(&self) -> usize {
        self.listings.len()
    }

    pub fn create_collection(
        &self,
        user_id: &str,
        request: CreateCollectionRequest,
    ) -> std::result::Result<SkillCollection, crate::utils::AetherisError> {
        let collection_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let collection = SkillCollection {
            collection_id: collection_id.clone(),
            name: request.name,
            description: request.description,
            author_id: user_id.to_string(),
            author_name: None,
            created_at: now,
            updated_at: now,
            skills: Vec::new(),
            is_public: request.is_public,
            follower_count: 0,
        };

        self.collections
            .insert(collection_id.clone(), collection.clone());

        self.user_collections
                .entry(user_id.to_string())
                .or_default()
                .push(collection_id.clone());

        info!("Created collection: {}", collection_id);

        self.save()?;
        Ok(collection)
    }

    pub fn update_collection(
        &self,
        collection_id: &str,
        user_id: &str,
        request: UpdateCollectionRequest,
    ) -> std::result::Result<SkillCollection, crate::utils::AetherisError> {
        let mut collection = self.collections.get_mut(collection_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Collection not found: {}", collection_id))
        })?;

        if collection.author_id != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to update this collection".to_string(),
            ));
        }

        if let Some(name) = request.name {
            collection.name = name;
        }
        if let Some(description) = request.description {
            collection.description = description;
        }
        if let Some(is_public) = request.is_public {
            collection.is_public = is_public;
        }
        collection.updated_at = chrono::Utc::now();

        let updated_collection = collection.value().clone();
        self.save()?;
        Ok(updated_collection)
    }

    pub fn delete_collection(
        &self,
        collection_id: &str,
        user_id: &str,
    ) -> std::result::Result<(), crate::utils::AetherisError> {
        let collection = self.collections.get(collection_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Collection not found: {}", collection_id))
        })?;

        if collection.author_id != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to delete this collection".to_string(),
            ));
        }

        self.collections.remove(collection_id);

        if let Some(mut user_collections) = self.user_collections.get_mut(user_id) {
            user_collections.retain(|id| id != collection_id);
        }

        info!("Deleted collection: {}", collection_id);

        self.save()?;
        Ok(())
    }

    pub fn add_skill_to_collection(
        &self,
        collection_id: &str,
        user_id: &str,
        listing_id: &str,
    ) -> std::result::Result<SkillCollection, crate::utils::AetherisError> {
        if !self.listings.contains_key(listing_id) {
            return Err(AetherisError::NotFound(format!(
                "Listing not found: {}",
                listing_id
            )));
        }

        let mut collection = self.collections.get_mut(collection_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Collection not found: {}", collection_id))
        })?;

        if collection.author_id != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to modify this collection".to_string(),
            ));
        }

        if !collection.skills.contains(&listing_id.to_string()) {
            collection.skills.push(listing_id.to_string());
            collection.updated_at = chrono::Utc::now();
        }

        let updated_collection = collection.value().clone();
        self.save()?;
        Ok(updated_collection)
    }

    pub fn remove_skill_from_collection(
        &self,
        collection_id: &str,
        user_id: &str,
        listing_id: &str,
    ) -> std::result::Result<SkillCollection, crate::utils::AetherisError> {
        let mut collection = self.collections.get_mut(collection_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Collection not found: {}", collection_id))
        })?;

        if collection.author_id != user_id {
            return Err(AetherisError::Validation(
                "Not authorized to modify this collection".to_string(),
            ));
        }

        collection.skills.retain(|id| id != listing_id);
        collection.updated_at = chrono::Utc::now();

        let updated_collection = collection.value().clone();
        self.save()?;
        Ok(updated_collection)
    }

    pub fn get_collections(&self, user_id: &str) -> Vec<SkillCollection> {
        self.collections
            .iter()
            .filter(|entry| {
                let collection = entry.value();
                collection.author_id == user_id || collection.is_public
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_collection(&self, collection_id: &str, user_id: &str) -> Option<SkillCollection> {
        let collection = self.collections.get(collection_id)?;
        if collection.author_id == user_id || collection.is_public {
            Some(collection.value().clone())
        } else {
            None
        }
    }

    pub fn create_user_collection(
        &self,
        user_id: &str,
        request: CreateUserCollectionRequest,
    ) -> std::result::Result<SkillCollection, crate::utils::AetherisError> {
        let collection_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let collection = SkillCollection {
            collection_id: collection_id.clone(),
            name: request.name,
            description: request.description,
            author_id: user_id.to_string(),
            author_name: None,
            created_at: now,
            updated_at: now,
            skills: Vec::new(),
            is_public: false,
            follower_count: 0,
        };

        self.collections
            .insert(collection_id.clone(), collection.clone());

        self.user_collections
                .entry(user_id.to_string())
                .or_default()
                .push(collection_id.clone());

        info!("Created user collection: {}", collection_id);

        self.save()?;
        Ok(collection)
    }

    pub fn add_to_user_favorites(
        &self,
        user_id: &str,
        listing_id: &str,
    ) -> std::result::Result<(), crate::utils::AetherisError> {
        if !self.listings.contains_key(listing_id) {
            return Err(AetherisError::NotFound(format!(
                "Listing not found: {}",
                listing_id
            )));
        }

        let favorites_collection_id = format!("favorites-{}", user_id);

        if !self.collections.contains_key(&favorites_collection_id) {
            let now = chrono::Utc::now();
            let favorites_collection = SkillCollection {
                collection_id: favorites_collection_id.clone(),
                name: "Favorites".to_string(),
                description: "My favorite skills".to_string(),
                author_id: user_id.to_string(),
                author_name: None,
                created_at: now,
                updated_at: now,
                skills: Vec::new(),
                is_public: false,
                follower_count: 0,
            };
            self.collections
                .insert(favorites_collection_id.clone(), favorites_collection);

            self.user_collections
                .entry(user_id.to_string())
                .or_default()
                .push(favorites_collection_id.clone());
        }

        if let Some(mut collection) = self.collections.get_mut(&favorites_collection_id) {
            if !collection.skills.contains(&listing_id.to_string()) {
                collection.skills.push(listing_id.to_string());
                collection.updated_at = chrono::Utc::now();
            }
        }

        info!("User {} added listing {} to favorites", user_id, listing_id);

        self.save()?;
        Ok(())
    }

    pub fn remove_from_user_favorites(
        &self,
        user_id: &str,
        listing_id: &str,
    ) -> std::result::Result<(), crate::utils::AetherisError> {
        let favorites_collection_id = format!("favorites-{}", user_id);

        if let Some(mut collection) = self.collections.get_mut(&favorites_collection_id) {
            collection.skills.retain(|id| id != listing_id);
            collection.updated_at = chrono::Utc::now();
        }

        info!(
            "User {} removed listing {} from favorites",
            user_id, listing_id
        );

        self.save()?;
        Ok(())
    }

    pub fn get_user_favorites(&self, user_id: &str) -> Vec<SkillListing> {
        let favorites_collection_id = format!("favorites-{}", user_id);

        if let Some(collection) = self.collections.get(&favorites_collection_id) {
            collection
                .skills
                .iter()
                .filter_map(|listing_id| self.listings.get(listing_id).map(|l| l.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }
}

pub fn create_marketplace_router(state: Arc<SkillMarketplaceState>) -> Router {
    Router::new()
        .route("/search", get(search_listings))
        .route("/popular", get(get_popular))
        .route("/top-rated", get(get_top_rated))
        .route("/featured", get(get_featured))
        .route("/listings", post(create_listing))
        .route(
            "/listings/:id",
            get(get_listing).put(update_listing).delete(delete_listing),
        )
        .route("/listings/:id/download", post(download_skill))
        .route("/listings/:id/reviews", get(get_reviews).post(add_review))
        .route("/collections", get(get_collections).post(create_collection))
        .route(
            "/collections/:id",
            get(get_collection)
                .put(update_collection)
                .delete(delete_collection),
        )
        .route(
            "/collections/:id/skills",
            post(add_skill_to_collection).delete(remove_skill_from_collection),
        )
        .route("/user/collections", post(create_user_collection))
        .route(
            "/user/favorites",
            get(get_user_favorites)
                .post(add_to_user_favorites)
                .delete(remove_from_user_favorites),
        )
        .route("/import-from-registry", post(import_from_registry))
        .route("/import-from-clawhub", post(import_from_clawhub))
        .with_state(state)
}

async fn search_listings(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> impl IntoResponse {
    let results = state.search_listings(query);
    Json(results)
}

async fn get_popular(State(state): State<Arc<SkillMarketplaceState>>) -> impl IntoResponse {
    let listings = state.get_popular_listings(20);
    Json(listings)
}

async fn get_top_rated(State(state): State<Arc<SkillMarketplaceState>>) -> impl IntoResponse {
    let listings = state.get_top_rated_listings(20);
    Json(listings)
}

async fn get_featured(State(state): State<Arc<SkillMarketplaceState>>) -> impl IntoResponse {
    let listings = state.get_featured_listings();
    Json(listings)
}

async fn get_listing(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> Result<Json<SkillListing>, StatusCode> {
    state
        .get_listing(&id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_listing(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<PublishRequest>,
) -> Result<Json<SkillListing>, StatusCode> {
    let user_id = "current-user";
    state
        .create_listing(user_id, request)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn download_skill(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> Result<Json<SkillListing>, StatusCode> {
    let user_id = "current-user";
    state
        .download_skill(&id, user_id)
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_reviews(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> impl IntoResponse {
    let reviews = state.get_reviews(&id);
    Json(reviews)
}

async fn add_review(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<ReviewRequest>,
) -> Result<Json<SkillReview>, StatusCode> {
    let user_id = "current-user";
    state
        .add_review(&id, user_id, request)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn create_collection(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<CreateCollectionRequest>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .create_collection(user_id, request)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_collections(State(state): State<Arc<SkillMarketplaceState>>) -> impl IntoResponse {
    let user_id = "current-user";
    let collections = state.get_collections(user_id);
    Json(collections)
}

async fn get_collection(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .get_collection(&id, user_id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn update_collection(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<UpdateCollectionRequest>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .update_collection(&id, user_id, request)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_collection(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> Result<StatusCode, StatusCode> {
    let user_id = "current-user";
    state
        .delete_collection(&id, user_id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_skill_to_collection(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<SkillToCollectionRequest>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .add_skill_to_collection(&id, user_id, &request.listing_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn remove_skill_from_collection(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<SkillToCollectionRequest>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .remove_skill_from_collection(&id, user_id, &request.listing_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn create_user_collection(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<CreateUserCollectionRequest>,
) -> Result<Json<SkillCollection>, StatusCode> {
    let user_id = "current-user";
    state
        .create_user_collection(user_id, request)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_to_user_favorites(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<UserFavoriteRequest>,
) -> Result<StatusCode, StatusCode> {
    let user_id = "current-user";
    state
        .add_to_user_favorites(user_id, &request.listing_id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn remove_from_user_favorites(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<UserFavoriteRequest>,
) -> Result<StatusCode, StatusCode> {
    let user_id = "current-user";
    state
        .remove_from_user_favorites(user_id, &request.listing_id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_user_favorites(State(state): State<Arc<SkillMarketplaceState>>) -> impl IntoResponse {
    let user_id = "current-user";
    let favorites = state.get_user_favorites(user_id);
    Json(favorites)
}

async fn update_listing(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<PublishRequest>,
) -> Result<Json<SkillListing>, StatusCode> {
    let user_id = "current-user";
    state
        .update_listing(&id, user_id, request)
        .map(Json)
        .map_err(|e| {
            error!("Failed to update listing: {}", e);
            match e {
                AetherisError::NotFound(_) => StatusCode::NOT_FOUND,
                AetherisError::Validation(_) => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })
}

async fn delete_listing(
    Path(id): Path<String>,
    State(state): State<Arc<SkillMarketplaceState>>,
) -> Result<StatusCode, StatusCode> {
    let user_id = "current-user";
    state
        .delete_listing(&id, user_id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            error!("Failed to delete listing: {}", e);
            match e {
                AetherisError::NotFound(_) => StatusCode::NOT_FOUND,
                AetherisError::Validation(_) => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })
}

async fn import_from_registry(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<ImportFromRegistryRequest>,
) -> Result<Json<SkillListing>, StatusCode> {
    info!(
        "Importing skill from registry: skill_id={}",
        request.skill_id
    );

    state
        .import_from_registry(&request.skill_id)
        .await
        .map(Json)
        .map_err(|e| {
            error!("Failed to import from registry: {}", e);
            match e {
                AetherisError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })
}

async fn import_from_clawhub(
    State(state): State<Arc<SkillMarketplaceState>>,
    Json(request): Json<ImportFromClawHubRequest>,
) -> Result<Json<SkillListing>, StatusCode> {
    info!(
        "Importing skill from ClawHub: skill_id={}, version={:?}",
        request.skill_id, request.version
    );

    state
        .import_from_clawhub(&request.skill_id, request.version.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            error!("Failed to import from ClawHub: {}", e);
            match e {
                AetherisError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })
}
