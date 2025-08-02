// Type-safe Tauri IPC communication composable

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  TauriCommands,
  TauriEvents,
  CommandName,
  CommandArgs,
  CommandResult,
  EventCallback,
} from "~/types/tauri";
import type { AppError, Result, createError } from "~/types/errors";

// Type-safe invoke wrapper
export const useTauriInvoke = () => {
  const invokeCommand = async <T extends CommandName>(
    command: T,
    ...args: CommandArgs<T>
  ): Promise<CommandResult<T>> => {
    try {
      // Handle commands with no arguments
      if (args.length === 0) {
        return await invoke(command);
      }

      // Handle commands with single argument
      if (args.length === 1) {
        return await invoke(command, args[0]);
      }

      // Handle commands with multiple arguments (convert to object)
      const argsObject = args.reduce((acc, arg, index) => {
        acc[`arg${index}`] = arg;
        return acc;
      }, {} as Record<string, any>);

      return await invoke(command, argsObject);
    } catch (error) {
      // Convert Tauri errors to our error types
      const appError = createError.communication(
        error instanceof Error ? error.message : String(error),
        command,
        false
      );
      throw appError;
    }
  };

  return { invoke: invokeCommand };
};

// Type-safe event listener wrapper
export const useTauriEvents = () => {
  const listenToEvent = async <T extends keyof TauriEvents>(
    event: T,
    callback: EventCallback<T>
  ): Promise<UnlistenFn> => {
    return await listen(event, (tauriEvent) => {
      callback(tauriEvent.payload as TauriEvents[T]);
    });
  };

  return { listen: listenToEvent };
};

// Main composable with all Tauri functionality
export const useTauri = () => {
  const { invoke: invokeCommand } = useTauriInvoke();
  const { listen: listenToEvent } = useTauriEvents();

  // Project Management
  const createProject = (
    config: Parameters<TauriCommands["create_project"]>[0]
  ) => invokeCommand("create_project", config);

  const getProject = (id: string) => invokeCommand("get_project", id);

  const getAllProjects = () => invokeCommand("get_all_projects");

  const updateProjectScanStatus = (
    id: string,
    status: Parameters<TauriCommands["update_project_scan_status"]>[1]
  ) => invokeCommand("update_project_scan_status", id, status);

  const deleteProject = (id: string) => invokeCommand("delete_project", id);

  const projectExists = (id: string) => invokeCommand("project_exists", id);

  // Asset Management
  const getAsset = (id: string) => invokeCommand("get_asset", id);

  const getAssetsByProject = (projectId: string) =>
    invokeCommand("get_assets_by_project", projectId);

  const getAssetsPaginated = (
    projectId: string,
    page: number,
    pageSize: number,
    filter?: Parameters<TauriCommands["get_assets_paginated"]>[3],
    sort?: Parameters<TauriCommands["get_assets_paginated"]>[4]
  ) =>
    invokeCommand(
      "get_assets_paginated",
      projectId,
      page,
      pageSize,
      filter,
      sort
    );

  const getAssetCount = (projectId: string) =>
    invokeCommand("get_asset_count", projectId);

  const getProjectTotalSize = (projectId: string) =>
    invokeCommand("get_project_total_size", projectId);

  const findDuplicates = (projectId: string) =>
    invokeCommand("find_duplicates", projectId);

  // Scanning and Processing
  const startScan = (projectId: string) =>
    invokeCommand("start_scan", projectId);

  const cancelScan = () => invokeCommand("cancel_scan");

  const getScanProgress = () => invokeCommand("get_scan_progress");

  const generateThumbnails = (projectId: string) =>
    invokeCommand("generate_thumbnails", projectId);

  const computeHashes = (projectId: string) =>
    invokeCommand("compute_hashes", projectId);

  // Variant Groups
  const getGroups = (projectId: string) =>
    invokeCommand("get_groups", projectId);

  const getGroupsByType = (projectId: string, groupType: "exact" | "similar") =>
    invokeCommand("get_groups_by_type", projectId, groupType);

  const getGroup = (groupId: string) => invokeCommand("get_group", groupId);

  const getGroupAssets = (groupId: string) =>
    invokeCommand("get_group_assets", groupId);

  const updateSuggestedKeep = (groupId: string, assetId?: string) =>
    invokeCommand("update_suggested_keep", groupId, assetId);

  const deleteGroup = (groupId: string) =>
    invokeCommand("delete_group", groupId);

  const getGroupStats = (projectId: string) =>
    invokeCommand("get_group_stats", projectId);

  // Decision Management
  const setDecision = (
    assetId: string,
    state: Parameters<TauriCommands["set_decision"]>[1],
    reason: Parameters<TauriCommands["set_decision"]>[2],
    notes?: string
  ) => invokeCommand("set_decision", assetId, state, reason, notes);

  const getDecision = (assetId: string) =>
    invokeCommand("get_decision", assetId);

  const getDecisionsByProject = (projectId: string) =>
    invokeCommand("get_decisions_by_project", projectId);

  const getDecisionsByState = (
    projectId: string,
    state: Parameters<TauriCommands["get_decisions_by_state"]>[1]
  ) => invokeCommand("get_decisions_by_state", projectId, state);

  const bulkSetDecisions = (
    request: Parameters<TauriCommands["bulk_set_decisions"]>[0]
  ) => invokeCommand("bulk_set_decisions", request);

  const clearDecisions = (projectId: string) =>
    invokeCommand("clear_decisions", projectId);

  const getDecisionStats = (projectId: string) =>
    invokeCommand("get_decision_stats", projectId);

  const getKeepAssets = (projectId: string) =>
    invokeCommand("get_keep_assets", projectId);

  const getRemoveAssets = (projectId: string) =>
    invokeCommand("get_remove_assets", projectId);

  // Similarity Detection
  const detectSimilar = (projectId: string, threshold: number) =>
    invokeCommand("detect_similar", projectId, threshold);

  const updateSimilarityThreshold = (projectId: string, threshold: number) =>
    invokeCommand("update_similarity_threshold", projectId, threshold);

  const computePerceptualHashes = (projectId: string) =>
    invokeCommand("compute_perceptual_hashes", projectId);

  // File Operations
  const previewOutput = (projectId: string, preserveStructure: boolean) =>
    invokeCommand("preview_output", projectId, preserveStructure);

  const copyKeepAssets = (projectId: string, preserveStructure: boolean) =>
    invokeCommand("copy_keep_assets", projectId, preserveStructure);

  const validateOutputPath = (outputPath: string, sourcePaths: string[]) =>
    invokeCommand("validate_output_path", outputPath, sourcePaths);

  const getDefaultOutputPath = (projectName: string) =>
    invokeCommand("get_default_output_path", projectName);

  // Export
  const exportManifestJson = (projectId: string) =>
    invokeCommand("export_manifest_json", projectId);

  const exportManifestCsv = (projectId: string) =>
    invokeCommand("export_manifest_csv", projectId);

  const getManifestData = (projectId: string) =>
    invokeCommand("get_manifest_data", projectId);

  // System
  const getAppVersion = () => invokeCommand("get_app_version");

  const getSupportedFormats = () => invokeCommand("get_supported_formats");

  const cleanupProjectData = (projectId: string) =>
    invokeCommand("cleanup_project_data", projectId);

  // Event Listeners
  const onScanProgress = (callback: EventCallback<"scan-progress">) =>
    listenToEvent("scan-progress", callback);

  const onThumbnailProgress = (callback: EventCallback<"thumbnail-progress">) =>
    listenToEvent("thumbnail-progress", callback);

  const onCopyProgress = (callback: EventCallback<"copy-progress">) =>
    listenToEvent("copy-progress", callback);

  const onScanComplete = (callback: EventCallback<"scan-complete">) =>
    listenToEvent("scan-complete", callback);

  const onScanCancelled = (callback: EventCallback<"scan-cancelled">) =>
    listenToEvent("scan-cancelled", callback);

  const onScanError = (callback: EventCallback<"scan-error">) =>
    listenToEvent("scan-error", callback);

  const onDecisionUpdated = (callback: EventCallback<"decision-updated">) =>
    listenToEvent("decision-updated", callback);

  const onGroupUpdated = (callback: EventCallback<"group-updated">) =>
    listenToEvent("group-updated", callback);

  return {
    // Raw invoke and listen functions
    invoke: invokeCommand,
    listen: listenToEvent,

    // Project Management
    createProject,
    getProject,
    getAllProjects,
    updateProjectScanStatus,
    deleteProject,
    projectExists,

    // Asset Management
    getAsset,
    getAssetsByProject,
    getAssetsPaginated,
    getAssetCount,
    getProjectTotalSize,
    findDuplicates,

    // Scanning and Processing
    startScan,
    cancelScan,
    getScanProgress,
    generateThumbnails,
    computeHashes,

    // Variant Groups
    getGroups,
    getGroupsByType,
    getGroup,
    getGroupAssets,
    updateSuggestedKeep,
    deleteGroup,
    getGroupStats,

    // Decision Management
    setDecision,
    getDecision,
    getDecisionsByProject,
    getDecisionsByState,
    bulkSetDecisions,
    clearDecisions,
    getDecisionStats,
    getKeepAssets,
    getRemoveAssets,

    // Similarity Detection
    detectSimilar,
    updateSimilarityThreshold,
    computePerceptualHashes,

    // File Operations
    previewOutput,
    copyKeepAssets,
    validateOutputPath,
    getDefaultOutputPath,

    // Export
    exportManifestJson,
    exportManifestCsv,
    getManifestData,

    // System
    getAppVersion,
    getSupportedFormats,
    cleanupProjectData,

    // Event Listeners
    onScanProgress,
    onThumbnailProgress,
    onCopyProgress,
    onScanComplete,
    onScanCancelled,
    onScanError,
    onDecisionUpdated,
    onGroupUpdated,
  };
};

// Error handling composable
export const useTauriErrorHandler = () => {
  const handleError = (error: unknown, context?: string): AppError => {
    if (error && typeof error === "object" && "message" in error) {
      return createError.communication(
        (error as Error).message,
        context,
        false
      );
    }

    return createError.communication(String(error), context, false);
  };

  const withErrorHandling = async <T>(
    operation: () => Promise<T>,
    context?: string
  ): Promise<Result<T, AppError>> => {
    try {
      const data = await operation();
      return { success: true, data };
    } catch (error) {
      return { success: false, error: handleError(error, context) };
    }
  };

  return {
    handleError,
    withErrorHandling,
  };
};
