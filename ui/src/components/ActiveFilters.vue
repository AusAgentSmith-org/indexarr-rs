<script setup lang="ts">
import type { SearchFilters } from '@/api'

const props = defineProps<{
  filters: SearchFilters
}>()

const emit = defineEmits<{
  remove: [key: keyof SearchFilters]
  clear: []
}>()

const filterLabels: Record<string, string> = {
  content_type: 'Type',
  resolution: 'Resolution',
  codec: 'Codec',
  video_source: 'Video Source',
  hdr: 'HDR',
  year: 'Year',
  source: 'Source',
  audio_codec: 'Audio',
  modifier: 'Release',
  platform: 'Platform',
  music_format: 'Format',
  has_subtitles: 'Subtitles',
  language: 'Language',
  category: 'Category',
}

function activeFilters(): { key: keyof SearchFilters; label: string; value: string }[] {
  const result: { key: keyof SearchFilters; label: string; value: string }[] = []
  for (const [key, value] of Object.entries(props.filters)) {
    if (value) {
      result.push({
        key: key as keyof SearchFilters,
        label: filterLabels[key] || key,
        value,
      })
    }
  }
  return result
}
</script>

<template>
  <div class="active-filters" v-if="activeFilters().length > 0">
    <span
      v-for="filter in activeFilters()"
      :key="filter.key"
      class="filter-pill"
    >
      <span class="filter-label">{{ filter.label }}:</span>
      <span class="filter-value">{{ filter.value }}</span>
      <button class="filter-remove" @click="emit('remove', filter.key)" title="Remove filter">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5">
          <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </span>
    <button class="clear-all" @click="emit('clear')" v-if="activeFilters().length > 1">
      Clear all
    </button>
  </div>
</template>

<style scoped>
.active-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 8px 0;
}

.filter-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 999px;
  background: var(--surface);
  border: 1px solid var(--border);
  font-size: 0.75rem;
}

.filter-label {
  color: var(--text-secondary);
  font-weight: 500;
}

.filter-value {
  color: var(--text);
  font-weight: 600;
}

.filter-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  color: var(--text-secondary);
  margin-left: 2px;
  transition: all 0.15s ease;
}

.filter-remove:hover {
  background: var(--border);
  color: var(--text);
}

.clear-all {
  font-size: 0.75rem;
  color: var(--accent);
  padding: 4px 8px;
}

.clear-all:hover {
  text-decoration: underline;
}
</style>
