<template>
    <div class="container mx-auto p-6 space-y-8">
        <div class="text-center">
            <h1 class="text-3xl font-bold mb-2">Cullrs - Tauri Commands Test</h1>
            <p class="text-muted-foreground">Test the backend Rust services from the frontend</p>
        </div>

        <!-- Directory Scanner Test -->
        <Card>
            <CardHeader>
                <CardTitle>Directory Scanner</CardTitle>
                <CardDescription>Test the scan_directory command</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <div class="flex gap-2">
                    <Button @click="selectDirectory" :disabled="scanning">
                        Select Directory
                    </Button>
                    <Button @click="scanDirectory" :disabled="!selectedPath || scanning" variant="outline">
                        {{ scanning ? 'Scanning...' : 'Scan Directory' }}
                    </Button>
                </div>

                <div v-if="selectedPath" class="text-sm text-muted-foreground">
                    Selected: {{ selectedPath }}
                </div>

                <div v-if="scanResults.length > 0" class="space-y-2">
                    <h4 class="font-semibold">Found {{ scanResults.length }} images:</h4>
                    <div class="max-h-40 overflow-y-auto space-y-1">
                        <div v-for="(file, index) in scanResults" :key="index"
                            class="text-sm p-2 bg-muted rounded flex justify-between">
                            <span>{{ file.path }}</span>
                            <span class="text-muted-foreground">{{ formatFileSize(file.size) }}</span>
                        </div>
                    </div>
                </div>
            </CardContent>
        </Card>

        <!-- File Hash Test -->
        <Card>
            <CardHeader>
                <CardTitle>File Hash Computer</CardTitle>
                <CardDescription>Test the compute_file_hash command</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <div class="flex gap-2">
                    <Button @click="selectFile" :disabled="hashing">
                        Select Image File
                    </Button>
                    <Button @click="computeHash" :disabled="!selectedFile || hashing" variant="outline">
                        {{ hashing ? 'Computing...' : 'Compute Hash' }}
                    </Button>
                </div>

                <div v-if="selectedFile" class="text-sm text-muted-foreground">
                    Selected: {{ selectedFile }}
                </div>

                <div v-if="fileHash" class="space-y-2">
                    <h4 class="font-semibold">File Hash (SHA-256):</h4>
                    <div class="text-sm font-mono p-2 bg-muted rounded break-all">
                        {{ fileHash }}
                    </div>
                </div>
            </CardContent>
        </Card>

        <!-- Perceptual Hash Test -->
        <Card>
            <CardHeader>
                <CardTitle>Perceptual Hash</CardTitle>
                <CardDescription>Test the compute_perceptual_hash command</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <Button @click="computePerceptualHash" :disabled="!selectedFile || perceptualHashing" variant="outline">
                    {{ perceptualHashing ? 'Computing...' : 'Compute Perceptual Hash' }}
                </Button>

                <div v-if="perceptualHash" class="space-y-2">
                    <h4 class="font-semibold">Perceptual Hashes:</h4>
                    <div class="space-y-1 text-sm">
                        <div><strong>DHash:</strong> <span class="font-mono">{{ perceptualHash.dhash }}</span></div>
                        <div><strong>PHash:</strong> <span class="font-mono">{{ perceptualHash.phash }}</span></div>
                        <div><strong>AHash:</strong> <span class="font-mono">{{ perceptualHash.ahash }}</span></div>
                    </div>
                </div>
            </CardContent>
        </Card>

        <!-- Image Quality Scoring Test -->
        <Card>
            <CardHeader>
                <CardTitle>Image Quality Scoring</CardTitle>
                <CardDescription>Test the score_image_quality command</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <Button @click="scoreImage" :disabled="!selectedFile || scoring" variant="outline">
                    {{ scoring ? 'Analyzing...' : 'Score Image Quality' }}
                </Button>

                <div v-if="qualityScore" class="space-y-2">
                    <h4 class="font-semibold">Quality Analysis:</h4>
                    <div class="grid grid-cols-2 gap-4 text-sm">
                        <div>
                            <strong>Overall Score:</strong>
                            <span class="ml-2">{{ (qualityScore.overall * 100).toFixed(1) }}%</span>
                        </div>
                        <div>
                            <strong>Sharpness:</strong>
                            <span class="ml-2">{{ (qualityScore.sharpness * 100).toFixed(1) }}%</span>
                        </div>
                        <div>
                            <strong>Exposure:</strong>
                            <span class="ml-2">{{ (qualityScore.exposure * 100).toFixed(1) }}%</span>
                        </div>
                        <div>
                            <strong>Composition:</strong>
                            <span class="ml-2">{{ (qualityScore.composition * 100).toFixed(1) }}%</span>
                        </div>
                    </div>

                    <div v-if="qualityScore.technical_issues.length > 0" class="mt-2">
                        <strong>Technical Issues:</strong>
                        <ul class="list-disc list-inside text-sm text-muted-foreground mt-1">
                            <li v-for="issue in qualityScore.technical_issues" :key="issue">{{ issue }}</li>
                        </ul>
                    </div>
                </div>
            </CardContent>
        </Card>

        <!-- App Configuration Test -->
        <Card>
            <CardHeader>
                <CardTitle>App Configuration</CardTitle>
                <CardDescription>Test the get_app_config and save_app_config commands</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <div class="flex gap-2">
                    <Button @click="loadConfig" :disabled="loadingConfig" variant="outline">
                        {{ loadingConfig ? 'Loading...' : 'Load Config' }}
                    </Button>
                    <Button @click="saveConfig" :disabled="!appConfig || savingConfig" variant="outline">
                        {{ savingConfig ? 'Saving...' : 'Save Config' }}
                    </Button>
                </div>

                <div v-if="appConfig" class="space-y-2">
                    <h4 class="font-semibold">Current Configuration:</h4>
                    <div class="text-sm space-y-1">
                        <div><strong>Similarity Threshold:</strong> {{ appConfig.similarity_threshold }}</div>
                        <div><strong>Parallel Workers:</strong> {{ appConfig.parallel_workers }}</div>
                        <div><strong>Cache Enabled:</strong> {{ appConfig.cache_enabled ? 'Yes' : 'No' }}</div>
                        <div><strong>AI Features:</strong> {{ appConfig.ai_features_enabled ? 'Yes' : 'No' }}</div>
                        <div><strong>Supported Formats:</strong> {{ appConfig.supported_formats.join(', ') }}</div>
                    </div>
                </div>
            </CardContent>
        </Card>

        <!-- Error Display -->
        <div v-if="error" class="p-4 bg-destructive/10 border border-destructive rounded-lg">
            <h4 class="font-semibold text-destructive mb-2">Error:</h4>
            <p class="text-sm text-destructive">{{ error }}</p>
        </div>
    </div>
</template>

<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'

// Reactive state
const selectedPath = ref('')
const selectedFile = ref('')
const scanResults = ref([])
const fileHash = ref('')
const perceptualHash = ref(null)
const qualityScore = ref(null)
const appConfig = ref(null)
const error = ref('')

// Loading states
const scanning = ref(false)
const hashing = ref(false)
const perceptualHashing = ref(false)
const scoring = ref(false)
const loadingConfig = ref(false)
const savingConfig = ref(false)

// Directory selection and scanning
const selectDirectory = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
        })
        if (selected) {
            selectedPath.value = selected
            scanResults.value = []
            error.value = ''
        }
    } catch (err) {
        error.value = `Failed to select directory: ${err}`
    }
}

const scanDirectory = async () => {
    if (!selectedPath.value) return

    scanning.value = true
    error.value = ''

    try {
        const options = {
            recursive: true,
            max_depth: null,
            supported_formats: ['jpg', 'jpeg', 'png', 'tiff', 'tif']
        }

        const results = await invoke('scan_directory', {
            path: selectedPath.value,
            options
        })

        scanResults.value = results
        toast.success(`Found ${results.length} images`)
    } catch (err) {
        error.value = `Scan failed: ${err}`
        toast.error('Scan failed')
    } finally {
        scanning.value = false
    }
}

// File selection and hashing
const selectFile = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Images',
                extensions: ['jpg', 'jpeg', 'png', 'tiff', 'tif']
            }]
        })
        if (selected) {
            selectedFile.value = selected
            fileHash.value = ''
            perceptualHash.value = null
            qualityScore.value = null
            error.value = ''
        }
    } catch (err) {
        error.value = `Failed to select file: ${err}`
    }
}

const computeHash = async () => {
    if (!selectedFile.value) return

    hashing.value = true
    error.value = ''

    try {
        const hash = await invoke('compute_file_hash', {
            path: selectedFile.value
        })

        fileHash.value = hash
        toast.success('Hash computed successfully')
    } catch (err) {
        error.value = `Hash computation failed: ${err}`
        toast.error('Hash computation failed')
    } finally {
        hashing.value = false
    }
}

const computePerceptualHash = async () => {
    if (!selectedFile.value) return

    perceptualHashing.value = true
    error.value = ''

    try {
        const hash = await invoke('compute_perceptual_hash', {
            path: selectedFile.value
        })

        perceptualHash.value = hash
        toast.success('Perceptual hash computed')
    } catch (err) {
        error.value = `Perceptual hash computation failed: ${err}`
        toast.error('Perceptual hash computation failed')
    } finally {
        perceptualHashing.value = false
    }
}

const scoreImage = async () => {
    if (!selectedFile.value) return

    scoring.value = true
    error.value = ''

    try {
        const score = await invoke('score_image_quality', {
            path: selectedFile.value
        })

        qualityScore.value = score
        toast.success('Image quality analyzed')
    } catch (err) {
        error.value = `Quality scoring failed: ${err}`
        toast.error('Quality scoring failed')
    } finally {
        scoring.value = false
    }
}

// Configuration management
const loadConfig = async () => {
    loadingConfig.value = true
    error.value = ''

    try {
        const config = await invoke('get_app_config')
        appConfig.value = config
        toast.success('Configuration loaded')
    } catch (err) {
        error.value = `Failed to load config: ${err}`
        toast.error('Failed to load config')
    } finally {
        loadingConfig.value = false
    }
}

const saveConfig = async () => {
    if (!appConfig.value) return

    savingConfig.value = true
    error.value = ''

    try {
        await invoke('save_app_config', {
            config: appConfig.value
        })
        toast.success('Configuration saved')
    } catch (err) {
        error.value = `Failed to save config: ${err}`
        toast.error('Failed to save config')
    } finally {
        savingConfig.value = false
    }
}

// Utility functions
const formatFileSize = (bytes) => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
</script>