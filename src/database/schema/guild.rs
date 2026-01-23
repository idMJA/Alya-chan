use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Guild settings and configuration
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "guild")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    /// Locale settings
    pub locale: Option<String>,
    /// Prefix settings
    pub prefix: Option<String>,
    /// Setup settings - chatbot channel
    pub chatbot_channel_id: Option<String>,
    /// Global chat channel settings
    pub global_channel_id: Option<String>,
    /// Global chat webhook ID
    pub global_webhook_id: Option<String>,
    /// Global chat webhook token
    pub global_webhook_token: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
