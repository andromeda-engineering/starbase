//! SeaORM entity for the `projects` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::memory_entry::Entity")]
    MemoryEntries,
    #[sea_orm(has_many = "super::config_entry::Entity")]
    ConfigEntries,
}

impl Related<super::memory_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MemoryEntries.def()
    }
}

impl Related<super::config_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConfigEntries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
