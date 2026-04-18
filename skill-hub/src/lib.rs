pub mod api;
pub mod config;
pub mod constants;
pub mod db;
pub mod observability;
pub mod utils;

pub use utils::{Result, SkillHubError};

pub mod prelude {
    pub use crate::api::{AppState, create_router};
    pub use crate::config::AppConfig;
    pub use crate::constants::*;
    pub use crate::db::*;
    pub use crate::observability::*;
    pub use crate::utils::{Result, SkillHubError};
}
