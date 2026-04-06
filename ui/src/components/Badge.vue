<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  type: string | null | undefined
}>()

const colorMap: Record<string, string> = {
  movie: '#3498db',
  tv_show: '#2ecc71',
  music: '#9b59b6',
  game: '#e67e22',
  software: '#1abc9c',
  ebook: '#e74c3c',
  xxx: '#95a5a6',
  unknown: '#bdc3c7',
}

const labelMap: Record<string, string> = {
  movie: 'Movie',
  tv_show: 'TV',
  music: 'Music',
  game: 'Game',
  software: 'Software',
  ebook: 'Ebook',
  xxx: 'XXX',
  unknown: 'Unknown',
}

const color = computed(() => colorMap[props.type || 'unknown'] || '#bdc3c7')
const label = computed(() => {
  if (!props.type) return 'Unknown'
  return labelMap[props.type] || props.type.charAt(0).toUpperCase() + props.type.slice(1).replace(/_/g, ' ')
})
</script>

<template>
  <span
    class="badge"
    :style="{
      backgroundColor: color + '18',
      color: color,
      borderColor: color + '40',
    }"
  >
    {{ label }}
  </span>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  white-space: nowrap;
  border: 1px solid;
}
</style>
