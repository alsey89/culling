<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { SelectIcon, SelectTrigger, type SelectTriggerProps, useForwardProps } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<SelectTriggerProps & { class?: HTMLAttributes['class'], size?: 'sm' | 'default' }>(),
  { size: 'default' },
)

const delegatedProps = reactiveOmit(props, 'class', 'size')
const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <SelectTrigger data-slot="select-trigger" :data-size="size" v-bind="forwardedProps" :class="cn(
    `data-[placeholder]:text-muted-foreground [&_svg:not([class*='text-'])]:text-muted-foreground flex w-fit items-center justify-between gap-2 border-3 border-black dark:border-white bg-input px-4 py-3 text-sm font-medium whitespace-nowrap neo-shadow transition-all outline-none focus:translate-x-1 focus:translate-y-1 focus:shadow-none focus:ring-4 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-11 data-[size=sm]:h-9 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4`,
    props.class,
  )">
    <slot />
    <SelectIcon as-child>
      <Icon name="lucide:chevron-down" class="w-6 h-6 opacity-50" />
    </SelectIcon>
  </SelectTrigger>
</template>
