<template>
    <div class="project-detail-page min-h-screen bg-gray-50">
        <!-- Header -->
        <header class="bg-white border-b px-6 py-4">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-4">
                    <Button variant="ghost" size="sm" @click="$router.push('/projects')"
                        class="text-gray-600 hover:text-gray-900">
                        ← Back to Projects
                    </Button>
                    <div>
                        <h1 class="text-2xl font-semibold text-gray-900">
                            {{ project?.name || 'Loading...' }}
                        </h1>
                        <p v-if="project" class="text-sm text-gray-600">
                            {{ project.source_path }}
                        </p>
                    </div>
                </div>

                <!-- Project stats -->
                <div v-if="project && !isLoading" class="flex items-center gap-6 text-sm">
                    <div class="text-center">
                        <div class="font-semibold text-gray-900">{{ totalAssets }}</div>
                        <div class="text-gray-600">Total Images</div>
                    </div>
                    <div class="text-center">
                        <div class="font-semibold text-blue-600">{{ project.scan_status }}</div>
                        <div class="text-gray-600">Status</div>
                    </div>
                </div>
            </div>
        </header>

        <!-- Main content -->
        <main class="p-6">
            <!-- Loading state -->
            <div v-if="isLoading" class="space-y-6">
                <div class="text-center py-8">
                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-4"></div>
                    <p class="text-gray-600">Loading project...</p>
                </div>

                <!-- Loading skeleton for thumbnails -->
                <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 gap-4">
                    <div v-for="i in 24" :key="i" class="aspect-square bg-gray-200 rounded-lg animate-pulse"></div>
                </div>
            </div>

            <!-- Error state -->
            <div v-else-if="error" class="text-center py-12">
                <div class="text-red-600 mb-4">
                    <svg class="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z">
                        </path>
                    </svg>
                </div>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">Error Loading Project</h3>
                <p class="text-gray-600 mb-4">{{ error }}</p>
                <Button @click="loadProjectData" variant="outline">
                    Try Again
                </Button>
            </div>

            <!-- Empty state - no assets -->
            <div v-else-if="assets.length === 0 && project?.scan_status === 'completed'" class="text-center py-12">
                <div class="text-gray-400 mb-4">
                    <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z">
                        </path>
                    </svg>
                </div>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">No Images Found</h3>
                <p class="text-gray-600 mb-4">
                    No images were found in the selected directory. Make sure the folder contains supported image
                    formats.
                </p>
                <Button @click="$router.push('/projects')" variant="outline">
                    Back to Projects
                </Button>
            </div>

            <!-- Scan not started state -->
            <div v-else-if="project?.scan_status === 'not_started'" class="text-center py-12">
                <div class="text-blue-600 mb-4">
                    <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                    </svg>
                </div>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">Ready to Scan</h3>
                <p class="text-gray-600 mb-4">
                    Start scanning to discover and process images in your project folder.
                </p>
                <Button @click="startScan" :disabled="isScanning">
                    {{ isScanning ? 'Scanning...' : 'Start Scan' }}
                </Button>
            </div>

            <!-- Scan in progress state -->
            <div v-else-if="project?.scan_status === 'in_progress' || isScanning" class="text-center py-12">
                <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">Scanning Images</h3>
                <div v-if="scanProgress" class="space-y-2">
                    <p class="text-gray-600">
                        {{ scanProgress.current_file ? getFileName(scanProgress.current_file) : 'Processing...' }}
                    </p>
                    <div class="w-64 mx-auto bg-gray-200 rounded-full h-2">
                        <div class="bg-blue-600 h-2 rounded-full transition-all duration-300"
                            :style="{ width: `${(scanProgress.files_processed / Math.max(scanProgress.total_files, 1)) * 100}%` }">
                        </div>
                    </div>
                    <p class="text-sm text-gray-500">
                        {{ scanProgress.files_processed }} of {{ scanProgress.total_files }} files
                        <span v-if="scanProgress.phase"> • {{ scanProgress.phase }}</span>
                    </p>
                </div>
                <Button @click="cancelScan" variant="outline" class="mt-4">
                    Cancel Scan
                </Button>
            </div>

            <!-- Thumbnail grid - main content -->
            <div v-else-if="assets.length > 0" class="space-y-6">
                <!-- Grid controls -->
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <h2 class="text-lg font-semibold text-gray-900">
                            Images ({{ assets.length }})
                        </h2>
                        <div class="flex items-center gap-2">
                            <label class="text-sm text-gray-600">Grid size:</label>
                            <select v-model="gridSize" class="text-sm border border-gray-300 rounded px-2 py-1">
                                <option value="small">Small</option>
                                <option value="medium">Medium</option>
                                <option value="large">Large</option>
                            </select>
                        </div>
                    </div>

                    <div class="flex items-center gap-2">
                        <Button @click="refreshAssets" variant="outline" size="sm" :disabled="isRefreshing">
                            {{ isRefreshing ? 'Refreshing...' : 'Refresh' }}
                        </Button>
                    </div>
                </div>

                <!-- Thumbnail grid -->
                <div class="grid gap-4" :class="gridClasses">
                    <div v-for="asset in assets" :key="asset.id"
                        class="group relative bg-white rounded-lg shadow-sm border hover:shadow-md transition-shadow cursor-pointer"
                        @click="openImageModal(asset)">
                        <!-- Thumbnail image -->
                        <div class="aspect-square bg-gray-100 rounded-t-lg overflow-hidden">
                            <img :src="getThumbnailUrl(asset.id)" :alt="getFileName(asset.path)"
                                class="w-full h-full object-cover" @error="handleImageError" loading="lazy" />

                            <!-- Loading overlay for thumbnails -->
                            <div v-if="!imageLoaded[asset.id]"
                                class="absolute inset-0 bg-gray-100 flex items-center justify-center">
                                <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-gray-400"></div>
                            </div>
                        </div>

                        <!-- Asset metadata -->
                        <div class="p-3 space-y-1">
                            <h3 class="font-medium text-sm text-gray-900 truncate" :title="getFileName(asset.path)">
                                {{ getFileName(asset.path) }}
                            </h3>
                            <div class="text-xs text-gray-600 space-y-0.5">
                                <div class="flex justify-between">
                                    <span>{{ asset.width }} × {{ asset.height }}</span>
                                    <span>{{ formatFileSize(asset.size) }}</span>
                                </div>
                                <div v-if="getExifData(asset)?.takenAt" class="text-gray-500">
                                    {{ formatDate(getExifData(asset)?.takenAt) }}
                                </div>
                            </div>
                        </div>

                        <!-- Hover overlay with actions -->
                        <div
                            class="absolute inset-0 bg-black bg-opacity-0 group-hover:bg-opacity-20 transition-all duration-200 rounded-lg flex items-center justify-center opacity-0 group-hover:opacity-100">
                            <Button size="sm" variant="secondary" class="bg-white/90 hover:bg-white">
                                View Full Size
                            </Button>
                        </div>
                    </div>
                </div>

                <!-- Load more button (if using pagination) -->
                <div v-if="hasMoreAssets" class="text-center py-6">
                    <Button @click="loadMoreAssets" :disabled="isLoadingMore" variant="outline">
                        {{ isLoadingMore ? 'Loading...' : 'Load More Images' }}
                    </Button>
                </div>
            </div>
        </main>

        <!-- Full-size image modal -->
        <div v-if="selectedAsset" class="fixed inset-0 bg-black bg-opacity-90 flex items-center justify-center z-50"
            @click="closeImageModal">
            <div class="relative max-w-full max-h-full p-4">
                <!-- Close button -->
                <Button @click="closeImageModal" variant="ghost" size="sm"
                    class="absolute top-2 right-2 z-10 text-white hover:text-gray-300 bg-black/20 hover:bg-black/40">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12">
                        </path>
                    </svg>
                </Button>

                <!-- Image -->
                <img :src="getOriginalImageUrl(selectedAsset.id)" :alt="getFileName(selectedAsset.path)"
                    class="max-w-full max-h-full object-contain" @click.stop />

                <!-- Image info overlay -->
                <div class="absolute bottom-4 left-4 bg-black/60 text-white p-3 rounded-lg max-w-sm">
                    <h3 class="font-medium mb-1">{{ getFileName(selectedAsset.path) }}</h3>
                    <div class="text-sm space-y-1">
                        <div>{{ selectedAsset.width }} × {{ selectedAsset.height }}</div>
                        <div>{{ formatFileSize(selectedAsset.size) }}</div>
                        <div v-if="getExifData(selectedAsset)?.takenAt">
                            {{ formatDate(getExifData(selectedAsset)?.takenAt) }}
                        </div>
                    </div>
                </div>

                <!-- Navigation arrows (if multiple images) -->
                <Button v-if="assets.length > 1" @click.stop="previousImage" variant="ghost" size="sm"
                    class="absolute left-4 top-1/2 transform -translate-y-1/2 text-white hover:text-gray-300 bg-black/20 hover:bg-black/40">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7">
                        </path>
                    </svg>
                </Button>

                <Button v-if="assets.length > 1" @click.stop="nextImage" variant="ghost" size="sm"
                    class="absolute right-4 top-1/2 transform -translate-y-1/2 text-white hover:text-gray-300 bg-black/20 hover:bg-black/40">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                    </svg>
                </Button>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { useTauri } from '@/composables/useTauri'
import type { Asset, Project, ScanProgress, ExifData } from '@/types/database'

// Route and router
const route = useRoute()
const router = useRouter()

// Tauri composable
const {
    loadProject,
    getProjectAssets,
    getProjectAssetsPaginated,
    getAssetCount,
    scanProject,
    cancelScan: cancelScanCommand,
    onScanProgress,
    onScanComplete,
    onScanError,
    getThumbnailPath,
    formatFileSize,
    getFileName,
} = useTauri()

// Reactive state
const project = ref<Project | null>(null)
const assets = ref<Asset[]>([])
const selectedAsset = ref<Asset | null>(null)
const isLoading = ref(true)
const isRefreshing = ref(false)
const isScanning = ref(false)
const isLoadingMore = ref(false)
const error = ref<string | null>(null)
const scanProgress = ref<ScanProgress | null>(null)
const totalAssets = ref(0)
const gridSize = ref<'small' | 'medium' | 'large'>('medium')
const imageLoaded = ref<Record<string, boolean>>({})

// Pagination state
const currentPage = ref(0)
const pageSize = ref(50)
const hasMoreAssets = ref(false)

// Computed properties
const projectId = computed(() => route.params.id as string)

const gridClasses = computed(() => {
    const sizeClasses = {
        small: 'grid-cols-3 md:grid-cols-6 lg:grid-cols-8 xl:grid-cols-10',
        medium: 'grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8',
        large: 'grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6',
    }
    return sizeClasses[gridSize.value]
})

// Event listeners for scan progress
let unlistenProgress: (() => void) | null = null
let unlistenComplete: (() => void) | null = null
let unlistenError: (() => void) | null = null

// Methods
const loadProjectData = async () => {
    try {
        isLoading.value = true
        error.value = null

        // Load project details
        project.value = await loadProject(projectId.value)

        // Load assets if scan is completed
        if (project.value.scan_status === 'completed') {
            await loadAssets()
        }
    } catch (err) {
        console.error('Failed to load project:', err)
        error.value = err instanceof Error ? err.message : 'Failed to load project'
    } finally {
        isLoading.value = false
    }
}

const loadAssets = async (append = false) => {
    try {
        if (!append) {
            assets.value = []
            currentPage.value = 0
        }

        const offset = currentPage.value * pageSize.value
        const newAssets = await getProjectAssetsPaginated(
            projectId.value,
            pageSize.value,
            offset
        )

        if (append) {
            assets.value.push(...newAssets)
        } else {
            assets.value = newAssets
        }

        // Check if there are more assets
        hasMoreAssets.value = newAssets.length === pageSize.value

        // Get total count
        totalAssets.value = await getAssetCount(projectId.value)

        // Initialize image loaded state
        newAssets.forEach(asset => {
            imageLoaded.value[asset.id] = false
        })
    } catch (err) {
        console.error('Failed to load assets:', err)
        error.value = err instanceof Error ? err.message : 'Failed to load assets'
    }
}

const loadMoreAssets = async () => {
    if (isLoadingMore.value || !hasMoreAssets.value) return

    try {
        isLoadingMore.value = true
        currentPage.value++
        await loadAssets(true)
    } catch (err) {
        console.error('Failed to load more assets:', err)
        currentPage.value-- // Revert page increment on error
    } finally {
        isLoadingMore.value = false
    }
}

const refreshAssets = async () => {
    if (isRefreshing.value) return

    try {
        isRefreshing.value = true
        await loadAssets()
    } catch (err) {
        console.error('Failed to refresh assets:', err)
    } finally {
        isRefreshing.value = false
    }
}

const startScan = async () => {
    if (isScanning.value) return

    try {
        isScanning.value = true
        error.value = null
        await scanProject(projectId.value)
    } catch (err) {
        console.error('Failed to start scan:', err)
        error.value = err instanceof Error ? err.message : 'Failed to start scan'
        isScanning.value = false
    }
}

const cancelScan = async () => {
    try {
        await cancelScanCommand()
        isScanning.value = false
        scanProgress.value = null
    } catch (err) {
        console.error('Failed to cancel scan:', err)
    }
}

// Image modal methods
const openImageModal = (asset: Asset) => {
    selectedAsset.value = asset
}

const closeImageModal = () => {
    selectedAsset.value = null
}

const previousImage = () => {
    if (!selectedAsset.value || assets.value.length <= 1) return

    const currentIndex = assets.value.findIndex(a => a.id === selectedAsset.value!.id)
    const previousIndex = currentIndex > 0 ? currentIndex - 1 : assets.value.length - 1
    const previousAsset = assets.value[previousIndex]
    if (previousAsset) {
        selectedAsset.value = previousAsset
    }
}

const nextImage = () => {
    if (!selectedAsset.value || assets.value.length <= 1) return

    const currentIndex = assets.value.findIndex(a => a.id === selectedAsset.value!.id)
    const nextIndex = currentIndex < assets.value.length - 1 ? currentIndex + 1 : 0
    const nextAsset = assets.value[nextIndex]
    if (nextAsset) {
        selectedAsset.value = nextAsset
    }
}

// Utility methods
const getThumbnailUrl = (assetId: string): string => {
    // For now, return a placeholder. In a real implementation, this would
    // serve the thumbnail from the temp folder via a backend endpoint
    return getThumbnailPath(assetId)
}

const getOriginalImageUrl = (assetId: string): string => {
    // For now, return the same as thumbnail. In a real implementation,
    // this would serve the original image file
    return `/api/assets/${assetId}/original`
}

const handleImageError = (event: Event) => {
    const img = event.target as HTMLImageElement
    // Set a placeholder or error image
    img.src = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTQgMTZMMTAuNTg2IDkuNDE0QTIgMiAwIDAgMSAxMy40MTQgOS40MTRMTDE2IDE2TTYgMjBIMThBMiAyIDAgMCAwIDIwIDE4VjZBMiAyIDAgMCAwIDE4IDRINkEyIDIgMCAwIDAgNCA2VjE4QTIgMiAwIDAgMCA2IDIwWiIgc3Ryb2tlPSIjOTlBM0FGIiBzdHJva2Utd2lkdGg9IjIiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIvPgo8L3N2Zz4K'
}

const formatDate = (dateString: string | undefined): string => {
    if (!dateString) return 'Unknown date'
    try {
        return new Date(dateString).toLocaleDateString()
    } catch {
        return 'Unknown date'
    }
}

const getExifData = (asset: Asset): ExifData | null => {
    if (!asset.exif_data) return null
    try {
        return JSON.parse(asset.exif_data)
    } catch {
        return null
    }
}

// Keyboard navigation
const handleKeydown = (event: KeyboardEvent) => {
    if (!selectedAsset.value) return

    switch (event.key) {
        case 'Escape':
            closeImageModal()
            break
        case 'ArrowLeft':
            previousImage()
            break
        case 'ArrowRight':
            nextImage()
            break
    }
}

// Lifecycle hooks
onMounted(async () => {
    // Set up event listeners for scan progress
    unlistenProgress = await onScanProgress((progress) => {
        scanProgress.value = progress
    })

    unlistenComplete = await onScanComplete((completedProjectId) => {
        if (completedProjectId === projectId.value) {
            isScanning.value = false
            scanProgress.value = null
            // Reload project and assets
            loadProjectData()
        }
    })

    unlistenError = await onScanError((errorMessage) => {
        isScanning.value = false
        scanProgress.value = null
        error.value = errorMessage
    })

    // Add keyboard event listener
    document.addEventListener('keydown', handleKeydown)

    // Load initial data
    await loadProjectData()
})

onUnmounted(() => {
    // Clean up event listeners
    if (unlistenProgress) unlistenProgress()
    if (unlistenComplete) unlistenComplete()
    if (unlistenError) unlistenError()

    document.removeEventListener('keydown', handleKeydown)
})

// Set page title
useHead({
    title: computed(() => project.value ? `${project.value.name} - Cullrs` : 'Project - Cullrs')
})
</script>