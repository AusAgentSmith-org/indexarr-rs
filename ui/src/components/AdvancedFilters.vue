<script setup lang="ts">
import { ref } from 'vue'
import type { Facets, SearchFilters } from '@/api'

const props = defineProps<{
  facets: Facets | null
  filters: SearchFilters
}>()

const emit = defineEmits<{
  filter: [key: keyof SearchFilters, value: string | undefined]
}>()

const expanded = ref(false)

function handleSelect(key: keyof SearchFilters, event: Event) {
  const target = event.target as HTMLSelectElement
  const value = target.value || undefined
  emit('filter', key, value)
}

function handleCheckbox(key: keyof SearchFilters, event: Event) {
  const target = event.target as HTMLInputElement
  emit('filter', key, target.checked ? 'true' : undefined)
}
</script>

<template>
  <div class="advanced-filters">
    <button class="toggle-btn" @click="expanded = !expanded">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 4h18M3 12h18M3 20h18" v-if="!expanded" />
        <path d="M18 6L6 18M6 6l12 12" v-else />
      </svg>
      {{ expanded ? 'Hide Filters' : 'More Filters' }}
    </button>

    <div v-if="expanded" class="filter-grid">
      <!-- Resolution -->
      <div class="filter-group" v-if="facets?.resolution?.length">
        <label class="filter-group-label">Resolution</label>
        <select
          :value="filters.resolution || ''"
          @change="handleSelect('resolution', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.resolution" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Video Codec -->
      <div class="filter-group" v-if="facets?.codec?.length">
        <label class="filter-group-label">Video Codec</label>
        <select
          :value="filters.codec || ''"
          @change="handleSelect('codec', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.codec" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Video Source -->
      <div class="filter-group" v-if="facets?.video_source?.length">
        <label class="filter-group-label">Video Source</label>
        <select
          :value="filters.video_source || ''"
          @change="handleSelect('video_source', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.video_source" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- HDR -->
      <div class="filter-group" v-if="facets?.hdr?.length">
        <label class="filter-group-label">HDR</label>
        <select
          :value="filters.hdr || ''"
          @change="handleSelect('hdr', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.hdr" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Audio Codec -->
      <div class="filter-group" v-if="facets?.audio_codec?.length">
        <label class="filter-group-label">Audio Codec</label>
        <select
          :value="filters.audio_codec || ''"
          @change="handleSelect('audio_codec', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.audio_codec" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Language -->
      <div class="filter-group" v-if="facets?.language?.length">
        <label class="filter-group-label">Language</label>
        <select
          :value="filters.language || ''"
          @change="handleSelect('language', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.language" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Modifier -->
      <div class="filter-group" v-if="facets?.modifier?.length">
        <label class="filter-group-label">Release Type</label>
        <select
          :value="filters.modifier || ''"
          @change="handleSelect('modifier', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.modifier" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Platform -->
      <div class="filter-group" v-if="facets?.platform?.length">
        <label class="filter-group-label">Platform</label>
        <select
          :value="filters.platform || ''"
          @change="handleSelect('platform', $event)"
        >
          <option value="">Any</option>
          <option v-for="f in facets.platform" :key="f.value" :value="f.value">
            {{ f.value }} ({{ f.count }})
          </option>
        </select>
      </div>

      <!-- Has Subtitles -->
      <div class="filter-group filter-checkbox">
        <label>
          <input
            type="checkbox"
            :checked="filters.has_subtitles === 'true'"
            @change="handleCheckbox('has_subtitles', $event)"
          />
          Has Subtitles
        </label>
      </div>
    </div>
  </div>
</template>

<style scoped>
.advanced-filters {
  margin-bottom: 8px;
}

.toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  border-radius: var(--radius);
  transition: all 0.15s ease;
}

.toggle-btn:hover {
  background: var(--surface);
  color: var(--text);
}

.filter-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 12px 0;
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 140px;
}

.filter-group-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.filter-group select {
  padding: 6px 8px;
  font-size: 0.8125rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  color: var(--text);
}

.filter-checkbox {
  justify-content: flex-end;
}

.filter-checkbox label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  cursor: pointer;
}

.filter-checkbox input[type="checkbox"] {
  accent-color: var(--accent);
}
</style>
