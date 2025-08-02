// Enhanced scan progress tracking types for frontend

export interface ScanProgress {
  files_processed: number;
  total_files: number;
  current_file: string;
  estimated_time_remaining?: number; // seconds
  phase: ScanPhase;
}

export enum ScanPhase {
  Discovery = "Discovery",
  Processing = "Processing",
  ThumbnailGeneration = "ThumbnailGeneration",
  HashingAndExif = "HashingAndExif",
  Complete = "Complete",
}

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

      // Start the enhanced scan
      await invoke("scan_project_enhanced", { projectId });
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
