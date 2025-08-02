use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// Project models
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = projects)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub output_path: String,
    pub exclude_patterns: String, // JSON string
    pub file_types: String,       // JSON string
    pub scan_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub output_path: String,
    pub exclude_patterns: String,
    pub file_types: String,
    pub scan_status: String,
    pub created_at: String,
    pub updated_at: String,
}

// Asset models
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = assets)]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub hash: Option<String>,
    pub perceptual_hash: Option<String>,
    pub size: i32,
    pub width: i32,
    pub height: i32,
    pub exif_data: Option<String>, // JSON string
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = assets)]
pub struct NewAsset {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub hash: Option<String>,
    pub perceptual_hash: Option<String>,
    pub size: i32,
    pub width: i32,
    pub height: i32,
    pub exif_data: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// EXIF data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub taken_at: Option<DateTime<Utc>>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<u32>,
    pub aperture: Option<f32>,
    pub shutter_speed: Option<String>,
}

// Variant group models
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = variant_groups)]
pub struct VariantGroup {
    pub id: String,
    pub project_id: String,
    pub group_type: String,
    pub similarity: f32,
    pub suggested_keep: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = variant_groups)]
pub struct NewVariantGroup {
    pub id: String,
    pub project_id: String,
    pub group_type: String,
    pub similarity: f32,
    pub suggested_keep: Option<String>,
    pub created_at: String,
}

// Asset group membership
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = asset_groups)]
pub struct AssetGroup {
    pub asset_id: String,
    pub group_id: String,
}

// Decision models
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = decisions)]
pub struct Decision {
    pub asset_id: String,
    pub state: String,
    pub reason: String,
    pub notes: Option<String>,
    pub decided_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = decisions)]
pub struct NewDecision {
    pub asset_id: String,
    pub state: String,
    pub reason: String,
    pub notes: Option<String>,
    pub decided_at: String,
}

// Enums for type safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    NotStarted,
    InProgress,
    Completed,
    Cancelled,
    Failed(String),
}

impl From<String> for ScanStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "not_started" => ScanStatus::NotStarted,
            "in_progress" => ScanStatus::InProgress,
            "completed" => ScanStatus::Completed,
            "cancelled" => ScanStatus::Cancelled,
            s if s.starts_with("failed:") => {
                ScanStatus::Failed(s.strip_prefix("failed:").unwrap_or("").to_string())
            }
            _ => ScanStatus::NotStarted,
        }
    }
}

impl From<ScanStatus> for String {
    fn from(status: ScanStatus) -> Self {
        match status {
            ScanStatus::NotStarted => "not_started".to_string(),
            ScanStatus::InProgress => "in_progress".to_string(),
            ScanStatus::Completed => "completed".to_string(),
            ScanStatus::Cancelled => "cancelled".to_string(),
            ScanStatus::Failed(msg) => format!("failed:{}", msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupType {
    Exact,
    Similar,
}

impl From<String> for GroupType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "exact" => GroupType::Exact,
            "similar" => GroupType::Similar,
            _ => GroupType::Exact,
        }
    }
}

impl From<GroupType> for String {
    fn from(group_type: GroupType) -> Self {
        match group_type {
            GroupType::Exact => "exact".to_string(),
            GroupType::Similar => "similar".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionState {
    Keep,
    Remove,
    Undecided,
}

impl From<String> for DecisionState {
    fn from(s: String) -> Self {
        match s.as_str() {
            "keep" => DecisionState::Keep,
            "remove" => DecisionState::Remove,
            "undecided" => DecisionState::Undecided,
            _ => DecisionState::Undecided,
        }
    }
}

impl From<DecisionState> for String {
    fn from(state: DecisionState) -> Self {
        match state {
            DecisionState::Keep => "keep".to_string(),
            DecisionState::Remove => "remove".to_string(),
            DecisionState::Undecided => "undecided".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasonCode {
    ExactDuplicate,
    HigherResolution,
    NewerTimestamp,
    LargerFilesize,
    UserOverrideKeep,
    UserOverrideRemove,
    ManualNoReason,
}

impl From<String> for ReasonCode {
    fn from(s: String) -> Self {
        match s.as_str() {
            "exact_duplicate" => ReasonCode::ExactDuplicate,
            "higher_resolution" => ReasonCode::HigherResolution,
            "newer_timestamp" => ReasonCode::NewerTimestamp,
            "larger_filesize" => ReasonCode::LargerFilesize,
            "user_override_keep" => ReasonCode::UserOverrideKeep,
            "user_override_remove" => ReasonCode::UserOverrideRemove,
            "manual_no_reason" => ReasonCode::ManualNoReason,
            _ => ReasonCode::ManualNoReason,
        }
    }
}

impl From<ReasonCode> for String {
    fn from(reason: ReasonCode) -> Self {
        match reason {
            ReasonCode::ExactDuplicate => "exact_duplicate".to_string(),
            ReasonCode::HigherResolution => "higher_resolution".to_string(),
            ReasonCode::NewerTimestamp => "newer_timestamp".to_string(),
            ReasonCode::LargerFilesize => "larger_filesize".to_string(),
            ReasonCode::UserOverrideKeep => "user_override_keep".to_string(),
            ReasonCode::UserOverrideRemove => "user_override_remove".to_string(),
            ReasonCode::ManualNoReason => "manual_no_reason".to_string(),
        }
    }
}
