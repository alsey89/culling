// Tauri IPC command type definitions for type-safe frontend-backend communication

import type {
  Asset,
  Project,
  VariantGroup,
  Decision,
  CreateProjectForm,
  ScanProgress,
  ThumbnailProgress,
  CopyProgress,
  DecisionStats,
  GroupStats,
  ProjectStats,
  OutputMapping,
  CopyResult,
  ProjectManifest,
  BulkDecisionRequest,
  BulkOperationResult,
  AssetFilter,
  GroupFilter,
  SortOptions,
  AssetSortField,
  GroupSortField,
  PaginatedResult,
  DecisionState,
  ReasonCode,
  ScanStatus,
} from "./database";

// Project Management Commands
export interface ProjectCommands {
  create_project: (config: CreateProjectForm) => Promise<Project>;
  get_project: (id: string) => Promise<Project>;
  get_all_projects: () => Promise<Project[]>;
  update_project_scan_status: (
    id: string,
    status: ScanStatus
  ) => Promise<Project>;
  delete_project: (id: string) => Promise<boolean>;
  project_exists: (id: string) => Promise<boolean>;
}

// Asset Management Commands
export interface AssetCommands {
  get_asset: (id: string) => Promise<Asset>;
  get_assets_by_project: (projectId: string) => Promise<Asset[]>;
  get_assets_paginated: (
    projectId: string,
    page: number,
    pageSize: number,
    filter?: AssetFilter,
    sort?: SortOptions<AssetSortField>
  ) => Promise<PaginatedResult<Asset>>;
  get_asset_count: (projectId: string) => Promise<number>;
  get_project_total_size: (projectId: string) => Promise<number>;
  find_duplicates: (projectId: string) => Promise<Asset[][]>;
}

// Scanning and Processing Commands
export interface ScanCommands {
  start_scan: (projectId: string) => Promise<void>;
  cancel_scan: () => Promise<void>;
  get_scan_progress: () => Promise<ScanProgress | null>;
  generate_thumbnails: (projectId: string) => Promise<void>;
  compute_hashes: (projectId: string) => Promise<void>;
}

// Variant Group Commands
export interface GroupCommands {
  get_groups: (projectId: string) => Promise<VariantGroup[]>;
  get_groups_by_type: (
    projectId: string,
    groupType: "exact" | "similar"
  ) => Promise<VariantGroup[]>;
  get_group: (groupId: string) => Promise<VariantGroup>;
  get_group_assets: (groupId: string) => Promise<Asset[]>;
  update_suggested_keep: (
    groupId: string,
    assetId?: string
  ) => Promise<VariantGroup>;
  delete_group: (groupId: string) => Promise<boolean>;
  get_group_stats: (projectId: string) => Promise<GroupStats>;
}

// Decision Management Commands
export interface DecisionCommands {
  set_decision: (
    assetId: string,
    state: DecisionState,
    reason: ReasonCode,
    notes?: string
  ) => Promise<Decision>;
  get_decision: (assetId: string) => Promise<Decision | null>;
  get_decisions_by_project: (projectId: string) => Promise<Decision[]>;
  get_decisions_by_state: (
    projectId: string,
    state: DecisionState
  ) => Promise<Decision[]>;
  bulk_set_decisions: (
    request: BulkDecisionRequest
  ) => Promise<BulkOperationResult>;
  clear_decisions: (projectId: string) => Promise<number>;
  get_decision_stats: (projectId: string) => Promise<DecisionStats>;
  get_keep_assets: (projectId: string) => Promise<string[]>;
  get_remove_assets: (projectId: string) => Promise<string[]>;
}

// Similarity Detection Commands
export interface SimilarityCommands {
  detect_similar: (
    projectId: string,
    threshold: number
  ) => Promise<VariantGroup[]>;
  update_similarity_threshold: (
    projectId: string,
    threshold: number
  ) => Promise<VariantGroup[]>;
  compute_perceptual_hashes: (projectId: string) => Promise<void>;
}

// File Operations Commands
export interface FileCommands {
  preview_output: (
    projectId: string,
    preserveStructure: boolean
  ) => Promise<OutputMapping[]>;
  copy_keep_assets: (
    projectId: string,
    preserveStructure: boolean
  ) => Promise<CopyResult[]>;
  validate_output_path: (
    outputPath: string,
    sourcePaths: string[]
  ) => Promise<boolean>;
  get_default_output_path: (projectName: string) => Promise<string>;
}

// Export Commands
export interface ExportCommands {
  export_manifest_json: (projectId: string) => Promise<string>;
  export_manifest_csv: (projectId: string) => Promise<string>;
  get_manifest_data: (projectId: string) => Promise<ProjectManifest>;
}

// System Commands
export interface SystemCommands {
  get_app_version: () => Promise<string>;
  get_supported_formats: () => Promise<string[]>;
  cleanup_project_data: (projectId: string) => Promise<void>;
}

// Event types for Tauri event system
export interface TauriEvents {
  "scan-progress": ScanProgress;
  "thumbnail-progress": ThumbnailProgress;
  "copy-progress": CopyProgress;
  "scan-complete": { projectId: string };
  "scan-cancelled": { projectId: string };
  "scan-error": { projectId: string; error: string };
  "decision-updated": { assetId: string; decision: Decision };
  "group-updated": { groupId: string; group: VariantGroup };
}

// Combined command interface for type safety
export interface TauriCommands
  extends ProjectCommands,
    AssetCommands,
    ScanCommands,
    GroupCommands,
    DecisionCommands,
    SimilarityCommands,
    FileCommands,
    ExportCommands,
    SystemCommands {}

// Error types for Tauri commands
export interface TauriError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
}

// Generic response wrapper
export interface TauriResponse<T> {
  success: boolean;
  data?: T;
  error?: TauriError;
}

// Command invocation helper types
export type CommandName = keyof TauriCommands;
export type CommandArgs<T extends CommandName> = Parameters<TauriCommands[T]>;
export type CommandResult<T extends CommandName> = ReturnType<TauriCommands[T]>;

// Event listener types
export type EventCallback<T extends keyof TauriEvents> = (
  event: TauriEvents[T]
) => void;
export type UnlistenFn = () => void;

// Tauri invoke wrapper with better typing
export interface TauriInvoke {
  <T extends CommandName>(
    command: T,
    ...args: CommandArgs<T>
  ): CommandResult<T>;
}

// Event listener wrapper with better typing
export interface TauriListen {
  <T extends keyof TauriEvents>(
    event: T,
    callback: EventCallback<T>
  ): Promise<UnlistenFn>;
}

// Tauri API interface
export interface TauriAPI {
  invoke: TauriInvoke;
  listen: TauriListen;
  emit: <T extends keyof TauriEvents>(
    event: T,
    payload: TauriEvents[T]
  ) => Promise<void>;
}
