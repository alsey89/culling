<template>
    <div class="min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800">
        <!-- Header -->
        <header class="border-b bg-white/80 backdrop-blur-sm dark:bg-slate-900/80">
            <div class="container mx-auto px-6 py-4 flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <div
                        class="w-8 h-8 bg-gradient-to-r from-purple-500 to-pink-500 rounded-lg flex items-center justify-center">
                        <span class="text-white font-bold text-sm">C</span>
                    </div>
                    <h1 class="text-xl font-semibold text-slate-900 dark:text-white">Cullrs</h1>
                </div>
                <div class="text-sm text-slate-500 dark:text-slate-400">
                    Privacy-first photo culling
                </div>
            </div>
        </header>

        <!-- Main Content -->
        <main class="container mx-auto px-6 py-12">
            <div class="max-w-2xl mx-auto">
                <!-- Welcome Section -->
                <div class="text-center mb-12">
                    <h2 class="text-3xl font-bold text-slate-900 dark:text-white mb-4">
                        Welcome to Cullrs
                    </h2>
                    <p class="text-lg text-slate-600 dark:text-slate-300 mb-8">
                        Quickly cull large photo sets and remove duplicates with privacy-first, on-device processing.
                    </p>
                </div>

                <!-- Project Creation Card -->
                <Card class="p-8">
                    <CardHeader class="text-center pb-6">
                        <CardTitle class="text-xl">Create New Project</CardTitle>
                        <CardDescription>
                            Start by creating a project and selecting your source images
                        </CardDescription>
                    </CardHeader>

                    <CardContent class="space-y-6">
                        <div class="space-y-2">
                            <Label for="project-name">Project Name</Label>
                            <Input id="project-name" v-model="projectName" placeholder="e.g., Wedding Photos 2024"
                                class="w-full" />
                        </div>

                        <div class="space-y-2">
                            <Label for="source-dir">Source Directory</Label>
                            <div class="flex space-x-2">
                                <Input id="source-dir" v-model="sourceDir"
                                    placeholder="Select folder containing photos..." readonly class="flex-1" />
                                <Button @click="selectSourceDirectory" variant="outline">
                                    Browse
                                </Button>
                            </div>
                            <p class="text-sm text-slate-500 dark:text-slate-400">
                                Choose the folder containing photos you want to cull
                            </p>
                        </div>

                        <div class="space-y-2">
                            <Label for="output-dir">Output Directory</Label>
                            <div class="flex space-x-2">
                                <Input id="output-dir" v-model="outputDir" placeholder="Select output folder..."
                                    readonly class="flex-1" />
                                <Button @click="selectOutputDirectory" variant="outline">
                                    Browse
                                </Button>
                            </div>
                            <p class="text-sm text-slate-500 dark:text-slate-400">
                                Selected photos will be copied here
                            </p>
                        </div>
                    </CardContent>

                    <CardFooter class="pt-6">
                        <Button @click="createProject" class="w-full" :disabled="!canCreateProject || isCreating">
                            <span v-if="isCreating" class="flex items-center">
                                <Icon name="svg-spinners:6-dots-rotate" class="w-4 h-4 mr-2" />
                                Creating Project...
                            </span>
                            <span v-else>Create Project</span>
                        </Button>
                    </CardFooter>
                </Card>

                <!-- Recent Projects (placeholder) -->
                <div class="mt-12">
                    <h3 class="text-lg font-semibold text-slate-900 dark:text-white mb-4">
                        Recent Projects
                    </h3>
                    <div class="text-center py-8 text-slate-500 dark:text-slate-400">
                        No recent projects yet. Create your first project above.
                    </div>
                </div>
            </div>
        </main>
    </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { useRouter } from 'vue-router'

const router = useRouter()

// Form state
const projectName = ref('')
const sourceDir = ref('')
const outputDir = ref('')
const isCreating = ref(false)

// Computed properties
const canCreateProject = computed(() => {
    return projectName.value.trim() && sourceDir.value && outputDir.value
})

// Directory selection using Tauri dialog
const selectSourceDirectory = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select Source Directory',
            defaultPath: sourceDir.value || undefined
        })

        if (selected) {
            sourceDir.value = selected
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