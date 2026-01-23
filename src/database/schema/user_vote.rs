use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User vote and premium tracking
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_vote")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    /// User ID
    pub user_id: String,
    /// When the vote was recorded
    pub voted_at: DateTime<Utc>,
    /// When the vote/premium expires
    pub expires_at: DateTime<Utc>,
    /// Type of vote: "vote" or "regular" (premium)
    pub vote_type: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
