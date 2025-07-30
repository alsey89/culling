<template>
    <div class="min-h-screen bg-slate-50 dark:bg-slate-900">
        <!-- Header -->
        <header class="border-b bg-white dark:bg-slate-800 px-6 py-4">
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <Button @click="$router.back()" variant="ghost" size="sm">
                        <Icon name="material-symbols:arrow-back" class="w-4 h-4 mr-2" />
                        Back
                    </Button>
                    <div
                        class="w-8 h-8 bg-gradient-to-r from-purple-500 to-pink-500 rounded-lg flex items-center justify-center">
                        <span class="text-white font-bold text-sm">C</span>
                    </div>
                    <div>
                        <h1 class="font-semibold text-slate-900 dark:text-white">
                            {{ projectName || 'Cullrs Project' }}
                        </h1>
                        <p class="text-sm text-slate-500 dark:text-slate-400">
                            Photo culling workspace
                        </p>
                    </div>
                </div>

                <div class="flex items-center space-x-3">
                    <Badge v-if="scanProgress" :variant="scanProgress.is_complete ? 'default' : 'secondary'">
                        {{ scanProgress.is_complete ? 'Scan Complete' : 'Scanning...' }}
                    </Badge>
                    <Button @click="startScan" :disabled="isScanning" variant="outline">
                        <Icon name="svg-spinners:6-dots-rotate" class="w-4 h-4 mr-2" />
                        {{ isScanning ? 'Scanning...' : 'Start Scan' }}
                    </Button>
                </div>
            </div>
        </header>

        <!-- Main Content -->
        <main class="p-6">
            <!-- Scanning Progress -->
            <div v-if="scanProgress && !scanProgress.is_complete" class="mb-6">
                <Card class="p-6">
                    <div class="space-y-4">
                        <div class="flex items-center justify-between">
                            <h3 class="text-lg font-semibold">Scanning Images</h3>
                            <span class="text-sm text-slate-500">
                                {{ scanProgress.processed }} / {{ scanProgress.total }} images
                            </span>
                        </div>

                        <Progress :value="scanProgress.percentage" class="w-full" />

                        <div class="text-sm text-slate-600 dark:text-slate-400">
                            <span v-if="scanProgress.current_file">
                                Processing: {{ getFileName(scanProgress.current_file) }}
                            </span>
                            <span v-else>
                                Preparing to scan...
                            </span>
                        </div>
                    </div>
                </Card>
            </div>

            <!-- Project Stats -->
            <div v-if="scanProgress?.is_complete" class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                <Card class="p-4">
                    <div class="text-2xl font-bold text-slate-900 dark:text-white">
                        {{ scanProgress.total }}
                    </div>
                    <div class="text-sm text-slate-500 dark:text-slate-400">
                        Total Images
                    </div>
                </Card>

                <Card class="p-4">
                    <div class="text-2xl font-bold text-slate-900 dark:text-white">
                        0
                    </div>
                    <div class="text-sm text-slate-500 dark:text-slate-400">
                        Exact Duplicates
                    </div>
                </Card>

                <Card class="p-4">
                    <div class="text-2xl font-bold text-slate-900 dark:text-white">
                        0
                    </div>
                    <div class="text-sm text-slate-500 dark:text-slate-400">
                        Similar Groups
                    </div>
                </Card>

                <Card class="p-4">
                    <div class="text-2xl font-bold text-slate-900 dark:text-white">
                        0
                    </div>
                    <div class="text-sm text-slate-500 dark:text-slate-400">
                        Selected to Keep
                    </div>
                </Card>
            </div>

            <!-- Culling Interface Placeholder -->
            <div v-if="scanProgress?.is_complete" class="space-y-6">
                <div class="flex items-center justify-between">
                    <h2 class="text-xl font-semibold text-slate-900 dark:text-white">
                        Photo Gallery
                    </h2>
                    <div class="flex items-center space-x-2">
                        <Button variant="outline" size="sm">
                            <Icon name="material-symbols:grid-view" class="w-4 h-4 mr-2" />
                            Grid View
                        </Button>
                        <Button variant="outline" size="sm">
                            <Icon name="material-symbols:list" class="w-4 h-4 mr-2" />
                            List View
                        </Button>
                    </div>
                </div>

                <Card class="p-8">
                    <div class="text-center text-slate-500 dark:text-slate-400">
                        <Icon name="material-symbols:add-a-photo" class="w-16 h-16 mx-auto mb-4 opacity-50" />
                        <h3 class="text-lg font-medium mb-2">Photo Gallery Coming Soon</h3>
                        <p class="text-sm">
                            The culling interface will display your scanned images here.<br>
                            You'll be able to view, compare, and select photos to keep.
                        </p>
                    </div>
                </Card>
            </div>

            <!-- Empty State -->
            <div v-if="!scanProgress" class="text-center py-12">
                <Card class="p-8">
                    <Icon name="material-symbols:add-a-photo"
                        class="w-16 h-16 mx-auto mb-4 opacity-50 text-slate-400" />
                    <h3 class="text-lg font-medium text-slate-900 dark:text-white mb-2">
                        Ready to Start
                    </h3>
                    <p class="text-slate-500 dark:text-slate-400 mb-4">
                        Click "Start Scan" to begin analyzing your photos
                    </p>
                    <Button @click="startScan" :disabled="isScanning">
                        <Icon name="svg-spinners:6-dots-rotate" class="w-4 h-4 mr-2" />
                        Start Scanning Images
                    </Button>
                </Card>
            </div>
        </main>
    </div>
</template>

<script setup>
import { invoke } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'


// State
const projectName = ref('')
const scanProgress = ref(null)
const isScanning = ref(false)
let progressInterval = null

// Get filename from path
const getFileName = (path) => {
    return path.split('/').pop() || path
}

// Start scanning
const startScan = async () => {
    if (isScanning.value) return

    isScanning.value = true

    try {
        // Start the scan
        await invoke('scan_directory')

        // Start polling for progress
        startProgressPolling()

        toast.success('Scan started', {
            description: 'Analyzing images in your source directory'
        })

    } catch (error) {
        toast.error('Failed to start scan', {
            description: error.message
        })
        isScanning.value = false
    }
}

// Progress polling
const startProgressPolling = () => {
    progressInterval = setInterval(async () => {
        try {
            const progress = await invoke('get_scan_progress')
            scanProgress.value = progress

            if (progress.is_complete) {
                stopProgressPolling()
                isScanning.value = false
                toast.success('Scan completed!', {
                    description: `Processed ${progress.total} images`
                })
            }
        } catch (error) {
            console.error('Failed to get scan progress:', error)
        }
    }, 1000) // Poll every second
}

const stopProgressPolling = () => {
    if (progressInterval) {
        clearInterval(progressInterval)
        progressInterval = null
    }
}

// Lifecycle
onMounted(() => {
    // Try to get current progress if scan is already running
    invoke('get_scan_progress')
        .then(progress => {
            scanProgress.value = progress
            if (!progress.is_complete && progress.total > 0) {
                isScanning.value = true
                startProgressPolling()
            }
        })
        .catch(() => {
            // No progress available, that's fine
        })
})

onUnmounted(() => {
    stopProgressPolling()
})
</script>
