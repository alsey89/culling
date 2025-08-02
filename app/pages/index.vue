<template>
    <div class="min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800">
        <!-- Header -->
        <header class="border-b bg-white/80 backdrop-blur-sm dark:bg-slate-900/80">
            <div class="container mx-auto px-6 py-4 flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div
                        class="w-8 h-8 bg-gradient-to-r from-purple-500 to-pink-500 rounded-lg flex items-center justify-center">
                        <span class="text-white font-bold text-sm">C</span>
                    </div>
                    <h1 class="text-xl font-semibold text-slate-900 dark:text-white">Cullrs</h1>
                </div>
                <Badge variant="secondary" class="text-xs">
                    Privacy-first photo culling
                </Badge>
            </div>
        </header>

        <!-- Main Content -->
        <main class="container mx-auto px-6 py-12">
            <div class="max-w-4xl mx-auto">
                <!-- Welcome Section -->
                <div class="text-center mb-12">
                    <h2 class="text-4xl font-bold text-slate-900 dark:text-white mb-4">
                        Welcome to Cullrs
                    </h2>
                    <p class="text-lg text-slate-600 dark:text-slate-300 max-w-2xl mx-auto">
                        Efficiently manage large photo collections through automated duplicate detection and similarity
                        grouping.
                        All processing happens on your device for complete privacy.
                    </p>
                </div>

                <div class="grid lg:grid-cols-3 gap-8">
                    <!-- Project Creation Card -->
                    <div class="lg:col-span-2">
                        <Card>
                            <CardHeader>
                                <CardTitle class="flex items-center gap-2">
                                    <Icon name="lucide:folder-plus" class="w-5 h-5" />
                                    Create New Project
                                </CardTitle>
                                <CardDescription>
                                    Simply select a source folder to get started. We'll handle the rest automatically.
                                </CardDescription>
                            </CardHeader>

                            <CardContent class="space-y-6">
                                <!-- Source Folder Selection -->
                                <div class="space-y-2">
                                    <Label for="source-folder" class="text-sm">
                                        Source Folder
                                    </Label>
                                    <div class="flex gap-2">
                                        <Input id="source-folder" v-model="sourceFolder"
                                            placeholder="Select folder containing photos..." readonly class="flex-1"
                                            :class="{ 'border-red-500': errors.sourceFolder }" />
                                        <Button @click="selectSourceFolder">
                                            <Icon name="lucide:folder-open" class="w-4 h-4 mr-2" />
                                            Browse
                                        </Button>
                                    </div>
                                    <p v-if="errors.sourceFolder" class="text-sm text-red-500">
                                        {{ errors.sourceFolder }}
                                    </p>
                                    <p v-else class="text-sm text-slate-500 dark:text-slate-400">
                                        Choose the folder containing photos you want to cull
                                    </p>
                                </div>

                                <!-- Project Name (Auto-generated, editable) -->
                                <div class="space-y-2">
                                    <Label for="project-name" class="text-sm font-medium">
                                        Project Name
                                    </Label>
                                    <Input id="project-name" v-model="projectName"
                                        placeholder="Project name will be auto-generated..."
                                        :class="{ 'border-red-500': errors.projectName }" />
                                    <p v-if="errors.projectName" class="text-sm text-red-500">
                                        {{ errors.projectName }}
                                    </p>
                                    <p v-else class="text-sm text-slate-500 dark:text-slate-400">
                                        Auto-generated from folder name, but you can edit it
                                    </p>
                                </div>

                                <!-- Output Location (Read-only, auto-generated) -->
                                <div class="space-y-2">
                                    <Label class="text-sm font-medium">
                                        Output Location
                                    </Label>
                                    <div
                                        class="px-3 py-2 bg-slate-50 dark:bg-slate-800 border rounded-md text-sm text-slate-600 dark:text-slate-300">
                                        {{ outputPath || 'Will be auto-generated: Documents/Cullrs/{project-name}' }}
                                    </div>
                                    <p class="text-sm text-slate-500 dark:text-slate-400">
                                        Culled photos will be automatically organized here.
                                        <Button variant="link" class="p-0" @click="showOutputSettings = true">
                                            Change Default Location
                                        </Button>
                                    </p>
                                </div>

                                <!-- File Type Filters -->
                                <div class="space-y-2">
                                    <Label class="text-sm font-medium">
                                        Supported File Types
                                    </Label>
                                    <div class="flex flex-wrap gap-2">
                                        <Badge v-for="type in supportedFileTypes" :key="type" variant="secondary"
                                            class="text-xs">
                                            {{ type.toUpperCase() }}
                                        </Badge>
                                    </div>
                                    <p class="text-sm text-slate-500 dark:text-slate-400">
                                        Only these image formats will be processed
                                    </p>
                                </div>
                            </CardContent>

                            <CardFooter class="flex gap-3">
                                <Button @click="createProject" class="flex-1"
                                    :disabled="!canCreateProject || isCreating">
                                    <Icon v-if="isCreating" name="svg-spinners:6-dots-rotate" class="w-4 h-4 mr-2" />
                                    <Icon v-else name="lucide:play" class="w-4 h-4 mr-2" />
                                    {{ isCreating ? 'Creating Project...' : 'Create Project' }}
                                </Button>
                            </CardFooter>
                        </Card>
                    </div>

                    <!-- Recent Projects Sidebar -->
                    <div class="space-y-6">
                        <Card>
                            <CardHeader>
                                <CardTitle class="flex items-center gap-2 text-base">
                                    <Icon name="lucide:clock" class="w-4 h-4" />
                                    Recent Projects
                                </CardTitle>
                            </CardHeader>
                            <CardContent>
                                <div v-if="recentProjects && recentProjects.length === 0" class="text-center py-8">
                                    <Icon name="lucide:folder-x"
                                        class="w-12 h-12 mx-auto text-slate-300 dark:text-slate-600 mb-3" />
                                    <p class="text-sm text-slate-500 dark:text-slate-400">
                                        No recent projects yet
                                    </p>
                                    <p class="text-xs text-slate-400 dark:text-slate-500 mt-1">
                                        Create your first project to get started
                                    </p>
                                </div>
                                <div v-else class="space-y-2">
                                    <Button v-for="project in recentProjects" :key="project.id"
                                        @click="openProject(project)" variant="ghost"
                                        class="w-full justify-start text-left p-3 h-auto">
                                        <div class="flex-1 min-w-0">
                                            <div class="font-medium text-sm truncate">
                                                {{ project.name }}
                                            </div>
                                            <div class="text-xs text-slate-500 dark:text-slate-400 truncate">
                                                {{ formatPath(project.source_path) }}
                                            </div>
                                            <div class="text-xs text-slate-400 dark:text-slate-500">
                                                {{ formatDate(project.updated_at) }}
                                            </div>
                                        </div>
                                        <Icon name="lucide:chevron-right" class="w-4 h-4 text-slate-400" />
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>

                        <!-- Quick Stats Card -->
                        <Card>
                            <CardHeader>
                                <CardTitle class="flex items-center gap-2 text-base">
                                    <Icon name="lucide:info" class="w-4 h-4" />
                                    Features
                                </CardTitle>
                            </CardHeader>
                            <CardContent class="space-y-3">
                                <div class="flex items-center gap-3 text-sm">
                                    <Icon name="lucide:shield-check" class="w-4 h-4 text-green-500" />
                                    <span>100% local processing</span>
                                </div>
                                <div class="flex items-center gap-3 text-sm">
                                    <Icon name="lucide:copy" class="w-4 h-4 text-blue-500" />
                                    <span>Exact duplicate detection</span>
                                </div>
                                <div class="flex items-center gap-3 text-sm">
                                    <Icon name="lucide:eye" class="w-4 h-4 text-purple-500" />
                                    <span>Visual similarity grouping</span>
                                </div>
                                <div class="flex items-center gap-3 text-sm">
                                    <Icon name="lucide:hard-drive" class="w-4 h-4 text-orange-500" />
                                    <span>Safe, non-destructive operations</span>
                                </div>
                            </CardContent>
                        </Card>
                    </div>
                </div>
            </div>
        </main>

        <!-- Output Settings Dialog -->
        <Dialog v-model:open="showOutputSettings">
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Default Output Location</DialogTitle>
                    <DialogDescription>
                        Change where culled photos are automatically saved by default
                    </DialogDescription>
                </DialogHeader>
                <div class="space-y-4">
                    <div class="space-y-2">
                        <Label>Current Default Location</Label>
                        <div class="px-3 py-2 bg-slate-50 dark:bg-slate-800 border rounded-md text-sm">
                            {{ defaultOutputLocation }}
                        </div>
                    </div>
                    <div class="space-y-2">
                        <Label>New Default Location</Label>
                        <div class="flex gap-2">
                            <Input v-model="newDefaultLocation" readonly class="flex-1" />
                            <Button @click="selectNewDefaultLocation" variant="outline">
                                Browse
                            </Button>
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button @click="showOutputSettings = false" variant="outline">
                        Cancel
                    </Button>
                    <Button @click="saveOutputSettings" :disabled="!newDefaultLocation">
                        Save Changes
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>

<script setup>
definePageMeta({
    title: 'Home - Cullrs',
    description: 'Quickly cull large photo sets and remove duplicates with privacy-first, on-device processing.',
    layout: "none"
})
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { useRouter } from 'vue-router'

const router = useRouter()

// Form state
const projectName = ref('')
const sourceFolder = ref('')
const sourceDir = ref('')
const outputDir = ref('')
const outputPath = ref('')
const isCreating = ref(false)

// Form validation
const errors = ref({
    sourceFolder: '',
    projectName: ''
})

// Settings dialog
const showOutputSettings = ref(false)
const defaultOutputLocation = ref('')
const newDefaultLocation = ref('')

// Supported file types
const supportedFileTypes = ref(['jpg', 'jpeg', 'png', 'tiff', 'tif', 'bmp', 'webp', 'heic', 'raw', 'cr2', 'nef', 'arw'])

// Computed properties
const canCreateProject = computed(() => {
    return projectName.value.trim() && sourceFolder.value && outputPath.value
})

const recentProjects = ref([])

// Initialize default output location
onMounted(async () => {
    try {
        const _recentProjects = await invoke('get_recent_projects')
        recentProjects.value = _recentProjects
    } catch (error) {
        console.error('Failed to get load recent projects:', error)
    }
    try {
        const defaultLocation = await invoke('get_default_output_location')
        defaultOutputLocation.value = defaultLocation

        // Set the output path to the default location + project name if no output is set
        if (!outputPath.value && projectName.value) {
            outputPath.value = `${defaultLocation}/${projectName.value}`
        }
    } catch (error) {
        console.error('Failed to get default output location:', error)
        // Fallback to a reasonable default
        defaultOutputLocation.value = './Cullrs'
    }
})

// Directory selection using Tauri dialog
const selectSourceFolder = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select Source Directory',
            defaultPath: sourceFolder.value || undefined
        })

        if (selected) {
            sourceFolder.value = selected
            sourceDir.value = selected

            // Auto-generate project name from folder name
            const folderName = selected.split('/').pop()
            if (!projectName.value) {
                projectName.value = folderName.replace(/[_-]/g, ' ').replace(/\b\w/g, l => l.toUpperCase())
            }

            // Auto-generate output path
            outputPath.value = `${defaultOutputLocation.value}/${projectName.value || folderName}`
            outputDir.value = outputPath.value

            // Clear any previous errors
            errors.value.sourceFolder = ''

            toast.success('Source directory selected', {
                description: `Selected: ${selected}`
            })
        }
    } catch (error) {
        toast.error('Failed to select directory', {
            description: error.message
        })
    }
}

const selectOutputDirectory = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select Output Directory',
            defaultPath: outputDir.value || undefined
        })

        if (selected) {
            outputDir.value = selected
            toast.success('Output directory selected', {
                description: `Selected: ${selected}`
            })
        }
    } catch (error) {
        toast.error('Failed to select directory', {
            description: error.message
        })
    }
}

// Utility functions
const formatPath = (path) => {
    if (!path) return ''
    const parts = path.split('/')
    if (parts.length > 3) {
        return `.../${parts.slice(-2).join('/')}`
    }
    return path
}

const formatDate = (dateString) => {
    if (!dateString) return ''
    try {
        const date = new Date(dateString)
        const now = new Date()
        const diffInDays = Math.floor((now - date) / (1000 * 60 * 60 * 24))

        if (diffInDays === 0) return 'Today'
        if (diffInDays === 1) return 'Yesterday'
        if (diffInDays < 7) return `${diffInDays} days ago`
        if (diffInDays < 30) return `${Math.floor(diffInDays / 7)} weeks ago`
        return date.toLocaleDateString()
    } catch {
        return 'Unknown date'
    }
}

const openProject = (project) => {
    toast.info(`Opening project: ${project.name}`)
    // Navigate to project workspace with project data
    navigateTo(`/project/${project.id}`)
}

const selectNewDefaultLocation = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select New Default Output Location'
        })

        if (selected) {
            newDefaultLocation.value = selected
        }
    } catch (error) {
        toast.error('Failed to select directory', {
            description: error.message
        })
    }
}

const saveOutputSettings = () => {
    if (newDefaultLocation.value) {
        defaultOutputLocation.value = newDefaultLocation.value
        toast.success('Default output location updated')
        showOutputSettings.value = false
        newDefaultLocation.value = ''
    }
}

// Project creation
const createProject = async () => {
    if (!canCreateProject.value) return

    isCreating.value = true

    try {
        const config = await invoke('create_project', {
            sourceDir: sourceDir.value,
            outputDir: outputDir.value,
            projectName: projectName.value.trim()
        })

        toast.success('Project created successfully!', {
            description: `Created project: ${config.name}`
        })

        // Navigate to the project workspace
        router.push('/project')

    } catch (error) {
        toast.error('Failed to create project', {
            description: error.message || 'Unknown error occurred'
        })
    } finally {
        isCreating.value = false
    }
}
</script>