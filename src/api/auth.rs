use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    Operator,
    Viewer,
    Developer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    PipelineRead,
    PipelineWrite,
    PipelineDelete,
    PipelineExecute,
    AgentRead,
    AgentWrite,
    ConfigRead,
    ConfigWrite,
    AuditRead,
    UserManagement,
}

impl UserRole {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            UserRole::Admin => vec![
                Permission::PipelineRead,
                Permission::PipelineWrite,
                Permission::PipelineDelete,
                Permission::PipelineExecute,
                Permission::AgentRead,
                Permission::AgentWrite,
                Permission::ConfigRead,
                Permission::ConfigWrite,
                Permission::AuditRead,
                Permission::UserManagement,
            ],
            UserRole::Operator => vec![
                Permission::PipelineRead,
                Permission::PipelineWrite,
                Permission::PipelineExecute,
                Permission::AgentRead,
            ],
            UserRole::Viewer => vec![
                Permission::PipelineRead,
                Permission::AgentRead,
                Permission::AuditRead,
            ],
            UserRole::Developer => vec![
                Permission::PipelineRead,
                Permission::PipelineWrite,
                Permission::PipelineDelete,
                Permission::PipelineExecute,
                Permission::ConfigRead,
            ],
        }
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

impl Default for LoginResponse {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            token_type: "Bearer".to_string(),
            expires_in: 0,
            user: UserInfo::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            username: String::new(),
            role: UserRole::Viewer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
}

pub struct AuthManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    jwt_expiration_hours: i64,
    jwt_issuer: String,
    users: Arc<RwLock<Vec<User>>>,
}

impl AuthManager {
    pub fn new(
        secret_key: &[u8],
        jwt_expiration_hours: Option<i64>,
        jwt_issuer: Option<String>,
        initial_admin_username: Option<String>,
        initial_admin_password: Option<String>,
    ) -> Self {
        let encoding_key = EncodingKey::from_secret(secret_key);
        let decoding_key = DecodingKey::from_secret(secret_key);

        let jwt_expiration_hours = jwt_expiration_hours.unwrap_or(24);
        let jwt_issuer = jwt_issuer.unwrap_or_else(|| "aetheris-engine".to_string());

        let mut users = Vec::new();

        if let (Some(username), Some(password)) = (initial_admin_username, initial_admin_password) {
            users.push(User {
                user_id: Uuid::new_v4(),
                username,
                password_hash: hash_password(&password),
                role: UserRole::Admin,
            });
        }

        Self {
            encoding_key,
            decoding_key,
            jwt_expiration_hours,
            jwt_issuer,
            users: Arc::new(RwLock::new(users)),
        }
    }

    pub async fn user_count(&self) -> usize {
        self.users.read().await.len()
    }

    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AuthError> {
        let users = self.users.read().await;

        let user = users
            .iter()
            .find(|u| u.username == request.username)
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(&request.password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        let now = Utc::now();
        let exp = (now + Duration::hours(self.jwt_expiration_hours)).timestamp();

        let claims = Claims {
            sub: user.user_id.to_string(),
            exp,
            iat: now.timestamp(),
            iss: self.jwt_issuer.clone(),
            user_id: user.user_id,
            username: user.username.clone(),
            role: user.role.clone(),
        };

        let token = jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_expiration_hours * 3600,
            user: UserInfo {
                user_id: user.user_id,
                username: user.username.clone(),
                role: user.role.clone(),
            },
        })
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.jwt_issuer]);
        validation.validate_exp = true;

        let claims = jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &validation)?;

        Ok(claims.claims)
    }

    pub async fn add_user(
        &self,
        username: String,
        password: String,
        role: UserRole,
    ) -> Result<Uuid, AuthError> {
        let mut users = self.users.write().await;

        if users.iter().any(|u| u.username == username) {
            return Err(AuthError::UserAlreadyExists);
        }

        let user_id = Uuid::new_v4();
        users.push(User {
            user_id,
            username,
            password_hash: hash_password(&password),
            role,
        });

        Ok(user_id)
    }

    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.clone()
    }

    pub async fn update_user_role(
        &self,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<(), AuthError> {
        let mut users = self.users.write().await;

        if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
            user.role = new_role;
            Ok(())
        } else {
            Err(AuthError::UserNotFound)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Token invalid: {0}")]
    TokenError(#[from] jsonwebtoken::errors::Error),
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("User not found")]
    UserNotFound,
    #[error("Password hashing error: {0}")]
    HashError(#[from] bcrypt::BcryptError),
}

fn hash_password(password: &str) -> String {
    hash(password, DEFAULT_COST).expect("Failed to hash password")
}

fn verify_password(password: &str, hash: &str) -> bool {
    verify(password, hash).unwrap_or(false)
}
