// Type-safe error handling interfaces

// Base error interface
export interface BaseError {
  message: string;
  code?: string;
  timestamp?: string;
}

// Database errors
export interface DatabaseError extends BaseError {
  type: "database";
  operation?: string;
  table?: string;
}

// File system errors
export interface FileSystemError extends BaseError {
  type: "filesystem";
  path?: string;
  operation?: "read" | "write" | "delete" | "create" | "access";
}

// Scan errors
export interface ScanError extends BaseError {
  type: "scan";
  phase?: "discovery" | "thumbnails" | "hashing" | "grouping";
  currentFile?: string;
  filesProcessed?: number;
}

// Image processing errors
export interface ImageError extends BaseError {
  type: "image";
  operation?: "thumbnail" | "hash" | "exif" | "resize";
  format?: string;
}

// Validation errors
export interface ValidationError extends BaseError {
  type: "validation";
  field?: string;
  value?: unknown;
  constraint?: string;
}

// Network/IPC errors
export interface CommunicationError extends BaseError {
  type: "communication";
  command?: string;
  timeout?: boolean;
}

// Permission errors
export interface PermissionError extends BaseError {
  type: "permission";
  resource?: string;
  requiredPermission?: string;
}

// Configuration errors
export interface ConfigError extends BaseError {
  type: "config";
  setting?: string;
  expectedType?: string;
}

// Union type for all possible errors
export type AppError =
  | DatabaseError
  | FileSystemError
  | ScanError
  | ImageError
  | ValidationError
  | CommunicationError
  | PermissionError
  | ConfigError;

// Error severity levels
export type ErrorSeverity = "low" | "medium" | "high" | "critical";

// Enhanced error with context
export interface ContextualError extends BaseError {
  severity: ErrorSeverity;
  context?: Record<string, unknown>;
  userMessage?: string;
  technicalDetails?: string;
  recoverable?: boolean;
  retryable?: boolean;
  suggestions?: string[];
}

// Error result type for operations that can fail
export type Result<T, E = AppError> =
  | { success: true; data: T }
  | { success: false; error: E };

// Async result type
export type AsyncResult<T, E = AppError> = Promise<Result<T, E>>;

// Error handler function type
export type ErrorHandler<T = void> = (error: AppError) => T;

// Error recovery function type
export type ErrorRecovery<T> = (error: AppError) => Promise<T | null>;

// Error reporting interface
export interface ErrorReporter {
  report: (error: AppError, context?: Record<string, unknown>) => void;
  reportWithContext: (error: ContextualError) => void;
}

// Error boundary props for Vue components
export interface ErrorBoundaryProps {
  fallback?: (error: AppError) => any;
  onError?: ErrorHandler;
  recovery?: ErrorRecovery<any>;
}

// Common error codes
export const ErrorCodes = {
  // Database
  DB_CONNECTION_FAILED: "DB_CONNECTION_FAILED",
  DB_QUERY_FAILED: "DB_QUERY_FAILED",
  DB_MIGRATION_FAILED: "DB_MIGRATION_FAILED",

  // File System
  FILE_NOT_FOUND: "FILE_NOT_FOUND",
  FILE_ACCESS_DENIED: "FILE_ACCESS_DENIED",
  DIRECTORY_NOT_FOUND: "DIRECTORY_NOT_FOUND",
  DISK_FULL: "DISK_FULL",

  // Scanning
  SCAN_CANCELLED: "SCAN_CANCELLED",
  SCAN_TIMEOUT: "SCAN_TIMEOUT",
  UNSUPPORTED_FORMAT: "UNSUPPORTED_FORMAT",

  // Image Processing
  IMAGE_CORRUPT: "IMAGE_CORRUPT",
  THUMBNAIL_GENERATION_FAILED: "THUMBNAIL_GENERATION_FAILED",
  HASH_COMPUTATION_FAILED: "HASH_COMPUTATION_FAILED",

  // Validation
  INVALID_PATH: "INVALID_PATH",
  INVALID_PROJECT_NAME: "INVALID_PROJECT_NAME",
  INVALID_THRESHOLD: "INVALID_THRESHOLD",

  // Communication
  COMMAND_TIMEOUT: "COMMAND_TIMEOUT",
  COMMAND_NOT_FOUND: "COMMAND_NOT_FOUND",
  SERIALIZATION_ERROR: "SERIALIZATION_ERROR",

  // Permission
  INSUFFICIENT_PERMISSIONS: "INSUFFICIENT_PERMISSIONS",
  ADMIN_REQUIRED: "ADMIN_REQUIRED",

  // Configuration
  INVALID_CONFIG: "INVALID_CONFIG",
  MISSING_CONFIG: "MISSING_CONFIG",
} as const;

export type ErrorCode = (typeof ErrorCodes)[keyof typeof ErrorCodes];

// Error factory functions
export const createError = {
  database: (
    message: string,
    operation?: string,
    table?: string
  ): DatabaseError => ({
    type: "database",
    message,
    operation,
    table,
    timestamp: new Date().toISOString(),
  }),

  filesystem: (
    message: string,
    path?: string,
    operation?: FileSystemError["operation"]
  ): FileSystemError => ({
    type: "filesystem",
    message,
    path,
    operation,
    timestamp: new Date().toISOString(),
  }),

  scan: (
    message: string,
    phase?: ScanError["phase"],
    currentFile?: string,
    filesProcessed?: number
  ): ScanError => ({
    type: "scan",
    message,
    phase,
    currentFile,
    filesProcessed,
    timestamp: new Date().toISOString(),
  }),

  image: (
    message: string,
    operation?: ImageError["operation"],
    format?: string
  ): ImageError => ({
    type: "image",
    message,
    operation,
    format,
    timestamp: new Date().toISOString(),
  }),

  validation: (
    message: string,
    field?: string,
    value?: unknown,
    constraint?: string
  ): ValidationError => ({
    type: "validation",
    message,
    field,
    value,
    constraint,
    timestamp: new Date().toISOString(),
  }),

  communication: (
    message: string,
    command?: string,
    timeout?: boolean
  ): CommunicationError => ({
    type: "communication",
    message,
    command,
    timeout,
    timestamp: new Date().toISOString(),
  }),

  permission: (
    message: string,
    resource?: string,
    requiredPermission?: string
  ): PermissionError => ({
    type: "permission",
    message,
    resource,
    requiredPermission,
    timestamp: new Date().toISOString(),
  }),

  config: (
    message: string,
    setting?: string,
    expectedType?: string
  ): ConfigError => ({
    type: "config",
    message,
    setting,
    expectedType,
    timestamp: new Date().toISOString(),
  }),
};

// Helper functions for error handling
export const isError = (
  result: Result<any, any>
): result is { success: false; error: AppError } => {
  return !result.success;
};

export const isSuccess = <T>(
  result: Result<T, any>
): result is { success: true; data: T } => {
  return result.success;
};

export const unwrap = <T>(result: Result<T, any>): T => {
  if (isSuccess(result)) {
    return result.data;
  }
  throw new Error(`Attempted to unwrap failed result: ${result.error.message}`);
};

export const unwrapOr = <T>(result: Result<T, any>, defaultValue: T): T => {
  return isSuccess(result) ? result.data : defaultValue;
};

export const mapError = <T, E1, E2>(
  result: Result<T, E1>,
  mapper: (error: E1) => E2
): Result<T, E2> => {
  return isSuccess(result)
    ? result
    : { success: false, error: mapper(result.error) };
};

export const mapSuccess = <T1, T2, E>(
  result: Result<T1, E>,
  mapper: (data: T1) => T2
): Result<T2, E> => {
  return isSuccess(result)
    ? { success: true, data: mapper(result.data) }
    : result;
};
