<template>
    <div class="flex h-screen bg-background">
        <!-- Sidebar -->
        <aside class="w-60 bg-sidebar border-r-5 border-sidebar-border flex flex-col neo-shadow-lg">
            <!-- Header -->
            <div class="px-6 py-4 border-sidebar-border flex items-end min-h-[72px] gap-2">
                <Button variant="border" class="flex items-center gap-3 p-2 h-auto" @click="$router.push('/')">
                    <span class="text-secondary-foreground font-bold text-xl">📸</span>
                </Button>
                <h1 class="text-3xl font-black text-sidebar-foreground tracking-tight">PhotoCull</h1>
            </div>

            <!-- Filters Section -->
            <div class="flex-1 p-6 overflow-y-auto">
                <div class="space-y-8">
                    <div class="space-y-6">
                        <h3 class="text-sm font-black text-sidebar-foreground uppercase tracking-wider">Filters</h3>

                        <!-- Rating Filter -->
                        <div class="space-y-3">
                            <Label class="font-bold">Rating</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="All Ratings" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="all">All Ratings</SelectItem>
                                    <SelectItem value="5">5 Stars</SelectItem>
                                    <SelectItem value="4">4 Stars</SelectItem>
                                    <SelectItem value="3">3 Stars</SelectItem>
                                    <SelectItem value="2">2 Stars</SelectItem>
                                    <SelectItem value="1">1 Star</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <!-- Date Filter -->
                        <div class="space-y-3">
                            <Label class="font-bold">Date</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="Any Date" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="any">Any Date</SelectItem>
                                    <SelectItem value="today">Today</SelectItem>
                                    <SelectItem value="week">This Week</SelectItem>
                                    <SelectItem value="month">This Month</SelectItem>
                                    <SelectItem value="year">This Year</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <!-- Camera Filter -->
                        <div class="space-y-3">
                            <Label class="font-bold">Camera</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="All Cameras" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="all">All Cameras</SelectItem>
                                    <SelectItem value="canon-r5">Canon EOS R5</SelectItem>
                                    <SelectItem value="sony-a7r4">Sony A7R IV</SelectItem>
                                    <SelectItem value="nikon-d850">Nikon D850</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <!-- Lens Filter -->
                        <div class="space-y-3">
                            <Label class="font-bold">Lens</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="All Lenses" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="all">All Lenses</SelectItem>
                                    <SelectItem value="24-70">24-70mm f/2.8</SelectItem>
                                    <SelectItem value="85">85mm f/1.4</SelectItem>
                                    <SelectItem value="16-35">16-35mm f/2.8</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                    </div>

                    <!-- Sort Section -->
                    <div class="space-y-6">
                        <h3 class="text-sm font-black text-sidebar-foreground uppercase tracking-wider">Sort</h3>

                        <!-- Sort By -->
                        <div class="space-y-3">
                            <Label class="font-bold">Sort By</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="Capture Time" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="capture-time">Capture Time</SelectItem>
                                    <SelectItem value="rating">Rating</SelectItem>
                                    <SelectItem value="file-name">File Name</SelectItem>
                                    <SelectItem value="file-size">File Size</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <!-- Order -->
                        <div class="space-y-3">
                            <Label class="font-bold">Order</Label>
                            <Select>
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="Ascending" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="asc">Ascending</SelectItem>
                                    <SelectItem value="desc">Descending</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                    </div>
                </div>
            </div>
        </aside>

        <!-- Main Content Area -->
        <div class="flex-1 flex flex-col">
            <!-- Top Bar -->
            <div class="bg-card border-b-5 border-border px-6 py-6 neo-shadow">
                <div class="flex items-center justify-between">
                    <!-- Search Bar -->
                    <div class="flex-1 max-w-lg">
                        <div class="relative">
                            <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                <Icon name="lucide:search" class="w-6 h-6 text-muted-foreground" />
                            </div>
                            <Input placeholder="Search photos..." class="pl-12 pr-4 bg-input font-medium" />
                        </div>
                    </div>

                    <!-- Action Buttons -->
                    <div class="flex items-center gap-6 ml-8">
                        <!-- Import Button -->
                        <Button variant="outline" class="font-bold">
                            <Icon name="lucide:download" class="w-6 h-6" />
                            IMPORT
                        </Button>

                        <!-- Export Button -->
                        <Button variant="outline" class="font-bold">
                            <Icon name="lucide:upload" class="w-6 h-6" />
                            EXPORT
                        </Button>

                        <!-- View Toggle -->
                        <ToggleGroup v-model="viewMode" type="single" variant="outline" size="icon"
                            class="grid grid-cols-2">
                            <ToggleGroupItem value="grid" aria-label="Grid View">
                                <!-- <Icon name="lucide:grid-3x3" class="w-6 h-6" /> -->
                                <Button variant="outline" class="p-2">
                                    <Icon name="lucide:grid-3x3" class="w-8 h-8" />
                                </Button>
                            </ToggleGroupItem>
                            <ToggleGroupItem value="list" aria-label="List View">
                                <Button variant="outline" class="p-2">
                                    <Icon name="lucide:list" class="w-8 h-8" />
                                </Button>
                            </ToggleGroupItem>
                        </ToggleGroup>

                        <!-- Profile Avatar -->
                        <div
                            class="w-12 h-12 bg-accent border-3 border-black dark:border-white flex items-center justify-center neo-shadow">
                            <span class="text-accent-foreground text-lg font-black">U</span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Photo Grid Area - Slot for content -->
            <div class="flex-1 p-8 overflow-y-auto bg-background">
                <slot />
            </div>

            <!-- Bottom Bar -->
            <div class="bg-card border-t-5 border-border px-6 py-6 neo-shadow">
                <div class="flex items-center justify-between">
                    <!-- Selection Count -->
                    <div class="flex items-center gap-3 text-lg font-bold text-card-foreground">
                        <span>12 PHOTOS SELECTED</span>
                    </div>

                    <!-- Action Buttons -->
                    <div class="flex items-center gap-4">
                        <!-- Metadata Button -->
                        <Button variant="outline" class="font-bold">
                            <Icon name="lucide:info" class="w-6 h-6" />
                            METADATA
                        </Button>

                        <!-- Batch Operations Button -->
                        <Button variant="secondary" class="font-black text-lg px-8">
                            <Icon name="lucide:package" class="w-6 h-6" />
                            BATCH OPERATIONS
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup>
import { ref } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/components/ui/select'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'

// Reactive state for view mode
const viewMode = ref('grid')

// Component logic will go here
// You can add reactive data, computed properties, and methods as needed
</script>

<style scoped>
/* Additional custom styles if needed */
</style>