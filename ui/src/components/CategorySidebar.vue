<script setup lang="ts">
import { computed } from 'vue'
import type { FacetValue, Facets } from '@/api'

const props = defineProps<{
  categories: FacetValue[]
  selected?: string
  facets?: Facets | null
}>()

const emit = defineEmits<{
  select: [value: string | undefined]
  filter: [key: string, value: string | undefined]
}>()

const categoryLabels: Record<string, string> = {
  movie: 'Movies',
  tv_show: 'TV Shows',
  music: 'Music',
  game: 'Games',
  software: 'Software',
  ebook: 'Ebooks',
  comic: 'Comics',
  audiobook: 'Audiobooks',
  xxx: 'XXX',
  unknown: 'Unknown',
}

interface SubCategory {
  label: string
  filterKey: string
  filterValue: string
  count?: number
}

const subcategories = computed<SubCategory[]>(() => {
  if (!props.selected || !props.facets) return []

  if (props.selected === 'movie' || props.selected === 'tv_show') {
    const resolutions = props.facets.resolution || []
    const tiers: { label: string; values: string[] }[] = [
      { label: '4K / UHD', values: ['2160p', '4320p', '1440p'] },
      { label: '1080p', values: ['1080p'] },
      { label: '720p', values: ['720p'] },
      { label: 'SD', values: ['480p', '576p', '360p'] },
    ]
    const result: SubCategory[] = []
    for (const tier of tiers) {
      const count = resolutions
        .filter(r => tier.values.includes(r.value))
        .reduce((sum, r) => sum + r.count, 0)
      if (count > 0) {
        result.push({
          label: tier.label,
          filterKey: 'resolution',
          filterValue: tier.values[0],
          count,
        })
      }
    }
    // Anime sub-item for TV
    if (props.selected === 'tv_show') {
      result.push({
        label: 'Anime',
        filterKey: 'is_anime',
        filterValue: 'true',
      })
    }
    return result
  }

  if (props.selected === 'game') {
    const platforms = props.facets.platform || []
    return platforms
      .filter(p => p.count > 0)
      .map(p => ({
        label: p.value,
        filterKey: 'platform',
        filterValue: p.value,
        count: p.count,
      }))
  }

  if (props.selected === 'music') {
    const formats = props.facets.music_format || []
    return formats
      .filter(f => f.count > 0)
      .map(f => ({
        label: f.value === 'lossless' ? 'Lossless' : 'Lossy',
        filterKey: 'music_format',
        filterValue: f.value,
        count: f.count,
      }))
  }

  return []
})

function getLabel(value: string): string {
  return categoryLabels[value] || value.charAt(0).toUpperCase() + value.slice(1).replace(/_/g, ' ')
}

function handleClick(value: string) {
  if (props.selected === value) {
    emit('select', undefined)
  } else {
    emit('select', value)
  }
}

function handleSubClick(sub: SubCategory) {
  emit('filter', sub.filterKey, sub.filterValue)
}
</script>

<template>
  <aside class="category-sidebar">
    <h3 class="sidebar-title">Categories</h3>
    <ul class="category-list">
      <li
        v-for="cat in categories"
        :key="cat.value"
      >
        <div
          class="category-item"
          :class="{ active: selected === cat.value }"
          @click="handleClick(cat.value)"
        >
          <span class="category-label">{{ getLabel(cat.value) }}</span>
          <span class="category-count">{{ cat.count }}</span>
        </div>
        <!-- Subcategories -->
        <ul v-if="selected === cat.value && subcategories.length > 0" class="subcategory-list">
          <li
            v-for="sub in subcategories"
            :key="sub.filterKey + sub.filterValue"
            class="subcategory-item"
            @click.stop="handleSubClick(sub)"
          >
            <span class="subcategory-label">{{ sub.label }}</span>
            <span v-if="sub.count" class="category-count">{{ sub.count }}</span>
          </li>
        </ul>
      </li>
    </ul>
    <button
      v-if="selected"
      class="clear-filter"
      @click="emit('select', undefined)"
    >
      Clear filter
    </button>
  </aside>
</template>

<style scoped>
.category-sidebar {
  width: var(--sidebar-width);
  flex-shrink: 0;
  padding: 16px 0;
}

.sidebar-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  padding: 0 12px 8px;
}

.category-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.category-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-radius: var(--radius);
  cursor: pointer;
  font-size: 0.875rem;
  color: var(--text-secondary);
  transition: all 0.15s ease;
  margin: 1px 0;
}

.category-item:hover {
  background: var(--surface);
  color: var(--text);
}

.category-item.active {
  background: var(--accent);
  color: #fff;
}

.category-item.active .category-count {
  color: rgba(255, 255, 255, 0.8);
}

.category-label {
  font-weight: 500;
}

.category-count {
  font-size: 0.75rem;
  color: var(--text-secondary);
  background: var(--surface);
  padding: 1px 6px;
  border-radius: 999px;
  min-width: 20px;
  text-align: center;
}

.category-item.active .category-count {
  background: rgba(255, 255, 255, 0.2);
}

.subcategory-list {
  list-style: none;
  padding: 0;
  margin: 0 0 4px 0;
}

.subcategory-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px 6px 24px;
  cursor: pointer;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  transition: all 0.15s ease;
}

.subcategory-item:hover {
  background: var(--surface);
  color: var(--text);
}

.subcategory-label {
  font-weight: 400;
}

.clear-filter {
  display: block;
  width: 100%;
  padding: 8px 12px;
  margin-top: 8px;
  font-size: 0.8125rem;
  color: var(--accent);
  text-align: left;
  cursor: pointer;
}

.clear-filter:hover {
  text-decoration: underline;
}

@media (max-width: 640px) {
  .category-sidebar {
    display: none;
  }
}
</style>
