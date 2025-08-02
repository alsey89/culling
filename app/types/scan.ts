// Enhanced scan progress tracking types for frontend

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ScanProgress } from "./database";

// Re-export types from database.ts for consistency
export type { ScanProgress, ThumbnailProgress, ScanPhase } from "./database";

export interface ScanState {
  isScanning: boolean;
  progress: ScanProgress | null;
  canCancel: boolean;
  error: string | null;
}

// Example usage in a composable
export const useScanProgress = () => {
  const scanState = ref<ScanState>({
    isScanning: false,
    progress: null,
    canCancel: false,
    error: null,
  });

  const startScan = async (projectId: string) => {
    try {
      scanState.value.isScanning = true;
      scanState.value.canCancel = true;
      scanState.value.error = null;

      // Start the scan
      await invoke("start_scan", { projectId });
    } catch (error) {
      scanState.value.error = error as string;
    } finally {
      scanState.value.isScanning = false;
      scanState.value.canCancel = false;
    }
  };

  const cancelScan = async () => {
    if (scanState.value.canCancel) {
      try {
        await invoke("cancel_scan");
        scanState.value.canCancel = false;
      } catch (error) {
        console.error("Failed to cancel scan:", error);
      }
    }
  };

  // Listen for progress events
  const unlistenProgress = listen<ScanProgress>("scan-progress", (event) => {
    scanState.value.progress = event.payload;
  });

  const unlistenComplete = listen<string>("scan-complete", (event) => {
    scanState.value.isScanning = false;
    scanState.value.canCancel = false;
    scanState.value.progress = null;
    console.log("Scan completed for project:", event.payload);
  });

  const unlistenCancelled = listen<string>("scan-cancelled", (event) => {
    scanState.value.isScanning = false;
    scanState.value.canCancel = false;
    scanState.value.progress = null;
    console.log("Scan cancelled for project:", event.payload);
  });

  const unlistenError = listen<string>("scan-error", (event) => {
    scanState.value.isScanning = false;
    scanState.value.canCancel = false;
    scanState.value.error = event.payload;
    console.error("Scan error:", event.payload);
  });

  // Cleanup listeners
  onUnmounted(() => {
    unlistenProgress.then((fn) => fn());
    unlistenComplete.then((fn) => fn());
    unlistenCancelled.then((fn) => fn());
    unlistenError.then((fn) => fn());
  });

  return {
    scanState: readonly(scanState),
    startScan,
    cancelScan,
  };
};
