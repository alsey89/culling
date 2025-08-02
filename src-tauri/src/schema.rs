// @generated automatically by Diesel CLI.

diesel::table! {
    asset_groups (asset_id, group_id) {
        asset_id -> Text,
        group_id -> Text,
    }
}

diesel::table! {
    assets (id) {
        id -> Text,
        project_id -> Text,
        path -> Text,
        hash -> Nullable<Text>,
        perceptual_hash -> Nullable<Text>,
        size -> Integer,
        width -> Integer,
        height -> Integer,
        exif_data -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        thumbnail_path -> Nullable<Text>,
    }
}

diesel::table! {
    decisions (asset_id) {
        asset_id -> Text,
        state -> Text,
        reason -> Text,
        notes -> Nullable<Text>,
        decided_at -> Text,
    }
}

diesel::table! {
    projects (id) {
        id -> Text,
        name -> Text,
        source_path -> Text,
        output_path -> Text,
        exclude_patterns -> Text,
        file_types -> Text,
        scan_status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    variant_groups (id) {
        id -> Text,
        project_id -> Text,
        group_type -> Text,
        similarity -> Float,
        suggested_keep -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::joinable!(asset_groups -> assets (asset_id));
diesel::joinable!(asset_groups -> variant_groups (group_id));
diesel::joinable!(assets -> projects (project_id));
diesel::joinable!(decisions -> assets (asset_id));
diesel::joinable!(variant_groups -> assets (suggested_keep));
diesel::joinable!(variant_groups -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    asset_groups,
    assets,
    decisions,
    projects,
    variant_groups,
);
