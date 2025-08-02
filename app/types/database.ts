// Core data model interfaces matching Rust backend models

export interface Asset {
  id: string;
  projectId: string;
  path: string;
  hash?: string;
  perceptualHash?: string;
  size: number;
  width: number;
  height: number;
  exifData?: ExifData;
  createdAt: string;
  updatedAt: string;
}

export interface ExifData {
  takenAt?: string;
  camera?: string;
  lens?: string;
  iso?: number;
  aperture?: number;
  shutterSpeed?: string;
}

export interface Project {
  id: string;
  name: string;
  sourcePath: string;
  outputPath: string;
  excludePatterns: string[];
  fileTypes: string[];
  scanStatus: ScanStatus;
  createdAt: string;
  updatedAt: string;
}

export type ScanStatus =
  | "not_started"
  | "in_progress"
  | "completed"
  | "cancelled"
  | "failed";

export interface VariantGroup {
  id: string;
  projectId: string;
  groupType: GroupType;
  similarity: number;
  suggestedKeep?: string;
  createdAt: string;
  assets?: Asset[]; // Populated when needed
}

export type GroupType = "exact" | "similar";

export interface Decision {
  assetId: string;
  state: DecisionState;
  reason: ReasonCode;
  notes?: string;
  decidedAt: string;
}

export type DecisionState = "keep" | "remove" | "undecided";

export type ReasonCode =
  | "exact_duplicate"
  | "higher_resolution"
  | "newer_timestamp"
  | "larger_filesize"
  | "user_override_keep"
  | "user_override_remove"
  | "manual_no_reason";

// Statistics and aggregated data types
export interface DecisionStats {
  keep: number;
  remove: number;
  undecided: number;
  total: number;
}

export interface GroupStats {
  totalGroups: number;
  exactGroups: number;
  similarGroups: number;
  totalAssetsInGroups: number;
}

export interface ProjectStats {
  totalAssets: number;
  totalSize: number;
  duplicateGroups: number;
  similarGroups: number;
  decisions: DecisionStats;
}

// Progress tracking types
export interface ScanProgress {
  filesProcessed: number;
  totalFiles: number;
  currentFile: string;
  estimatedTimeRemaining?: number;
  phase: ScanPhase;
}

export type ScanPhase = "scanning" | "thumbnails" | "hashing" | "grouping";

export interface ThumbnailProgress {
  thumbnailsGenerated: number;
  totalThumbnails: number;
  currentFile: string;
  estimatedTimeRemaining?: number;
}

export interface CopyProgress {
  filesCompleted: number;
  totalFiles: number;
  currentFile: string;
  bytesTransferred: number;
  totalBytes: number;
}

// File operation types
export interface OutputMapping {
  assetId: string;
  sourcePath: string;
  outputPath: string;
}

export interface CopyResult {
  assetId: string;
  success: boolean;
  error?: string;
}

// Manifest export types
export interface ManifestEntry {
  groupId?: string;
  assetId: string;
  sourcePath: string;
  outputPath?: string;
  decisionState: DecisionState;
  decisionReason: ReasonCode;
  similarity?: number;
  fileSize: number;
  dimensions: {
    width: number;
    height: number;
  };
  exif?: ExifData;
}

export interface ProjectManifest {
  projectId: string;
  projectName: string;
  appVersion: string;
  exportedAt: string;
  statistics: ProjectStats;
  entries: ManifestEntry[];
}

// Form and UI types
export interface CreateProjectForm {
  name: string;
  sourcePath: string;
  outputPath: string;
  excludePatterns: string[];
  fileTypes: string[];
}

export interface ProjectSettings {
  defaultOutputLocation: string;
  thumbnailSize: number;
  similarityThreshold: number;
  autoSuggestKeep: boolean;
  preserveFolderStructure: boolean;
}

// Validation types
export interface ValidationError {
  field: string;
  message: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

// Pagination types
export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}

// Filter and search types
export interface AssetFilter {
  projectId?: string;
  groupId?: string;
  decisionState?: DecisionState;
  hasExif?: boolean;
  minSize?: number;
  maxSize?: number;
  fileTypes?: string[];
}

export interface GroupFilter {
  projectId?: string;
  groupType?: GroupType;
  minSimilarity?: number;
  maxSimilarity?: number;
  hasDecisions?: boolean;
}

// Sorting types
export type AssetSortField = "createdAt" | "size" | "path" | "width" | "height";
export type GroupSortField = "createdAt" | "similarity" | "assetCount";
export type SortDirection = "asc" | "desc";

export interface SortOptions<T extends string> {
  field: T;
  direction: SortDirection;
}

// Bulk operation types
export interface BulkDecisionRequest {
  assetIds: string[];
  state: DecisionState;
  reason: ReasonCode;
  notes?: string;
}

export interface BulkOperationResult {
  successful: string[];
  failed: Array<{
    assetId: string;
    error: string;
  }>;
}
