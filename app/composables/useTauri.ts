import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Asset, Project, ScanProgress } from "~/types/database";

export const useTauri = () => {
  // Project commands
  const loadProject = async (projectId: string): Promise<Project> => {
    return await invoke("load_project", { projectId });
  };

  const getRecentProjects = async (): Promise<Project[]> => {
    return await invoke("get_recent_projects");
  };

  // Asset commands
  const getProjectAssets = async (projectId: string): Promise<Asset[]> => {
    return await invoke("get_project_assets", { projectId });
  };

  const getProjectAssetsPaginated = async (
    projectId: string,
    limit: number,
    offset: number
  ): Promise<Asset[]> => {
    return await invoke("get_project_assets_paginated", {
      projectId,
      limit,
      offset,
    });
  };

  const getAssetCount = async (projectId: string): Promise<number> => {
    return await invoke("get_asset_count", { projectId });
  };

  // Scan commands
  const scanProject = async (projectId: string): Promise<void> => {
    return await invoke("scan_project_enhanced", { projectId });
  };

  const cancelScan = async (): Promise<void> => {
    return await invoke("cancel_scan");
  };

  // Event listeners
  const onScanProgress = (callback: (progress: ScanProgress) => void) => {
    return listen<ScanProgress>("scan-progress", (event) => {
      callback(event.payload);
    });
  };

  const onScanComplete = (callback: (projectId: string) => void) => {
    return listen<string>("scan-complete", (event) => {
      callback(event.payload);
    });
  };

  const onScanError = (callback: (error: string) => void) => {
    return listen<string>("scan-error", (event) => {
      callback(event.payload);
    });
  };

  // Utility functions
  const getThumbnailPath = (assetId: string): string => {
    // This will be the path to the thumbnail in the temp folder
    // For now, we'll use a placeholder URL that the backend will serve
    return `/api/thumbnails/${assetId}`;
  };

  const formatFileSize = (bytes: number): string => {
    const sizes = ["B", "KB", "MB", "GB"];
    if (bytes === 0) return "0 B";
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round((bytes / Math.pow(1024, i)) * 100) / 100 + " " + sizes[i];
  };

  const getFileName = (path: string): string => {
    return path.split("/").pop() || path.split("\\").pop() || path;
  };

  return {
    // Project commands
    loadProject,
    getRecentProjects,

    // Asset commands
    getProjectAssets,
    getProjectAssetsPaginated,
    getAssetCount,

    // Scan commands
    scanProject,
    cancelScan,

    // Event listeners
    onScanProgress,
    onScanComplete,
    onScanError,

    // Utility functions
    getThumbnailPath,
    formatFileSize,
    getFileName,
  };
};
