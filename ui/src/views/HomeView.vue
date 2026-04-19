<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useSearchStore } from '@/stores/search'
import { getRecent, searchTorrents } from '@/api'
import type { RecentResult, FacetValue, Facets } from '@/api'
import SearchBar from '@/components/SearchBar.vue'
import CategorySidebar from '@/components/CategorySidebar.vue'
import TorrentTable from '@/components/TorrentTable.vue'
import ActiveFilters from '@/components/ActiveFilters.vue'
import AdvancedFilters from '@/components/AdvancedFilters.vue'
import Spinner from '@/components/Spinner.vue'

const searchStore = useSearchStore()
const sentinel = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null

const recentTorrents = ref<RecentResult[]>([])
const recentLoading = ref(false)
const categories = ref<FacetValue[]>([])
const categoryFacets = ref<Facets | null>(null)
const recentSortField = ref('date')
const recentSortOrder = ref<'asc' | 'desc'>('desc')

const sortedRecentTorrents = computed(() => {
  const list = [...recentTorrents.value]
  const field = recentSortField.value
  const dir = recentSortOrder.value === 'asc' ? 1 : -1

  list.sort((a, b) => {
    let va: any, vb: any
    switch (field) {
      case 'name':
        va = (a.name || '').toLowerCase()
        vb = (b.name || '').toLowerCase()
        return va < vb ? -dir : va > vb ? dir : 0
      case 'content_type':
        va = (a.content_type || '').toLowerCase()
        vb = (b.content_type || '').toLowerCase()
        return va < vb ? -dir : va > vb ? dir : 0
      case 'size':
        return ((a.size || 0) - (b.size || 0)) * dir
      case 'seeders':
        return ((a.seed_count || 0) - (b.seed_count || 0)) * dir
      case 'leechers':
        return ((a.peer_count || 0) - (b.peer_count || 0)) * dir
      case 'date':
        va = a.resolved_at || ''
        vb = b.resolved_at || ''
        return va < vb ? -dir : va > vb ? dir : 0
      default:
        return 0
    }
  })
  return list
})

async function loadRecent() {
  recentLoading.value = true
  try {
    const response = await getRecent(50)
    recentTorrents.value = response.results
  } catch {
    // silent fail
  } finally {
    recentLoading.value = false
  }
}

async function loadCategories() {
  try {
    const response = await searchTorrents('', {}, 'date', 'desc', 1, 0, true)
    if (response.facets) {
      categories.value = response.facets.content_type
    }
  } catch {
    // silent fail
  }
}

async function loadCategoryFacets(contentType: string) {
  try {
    const response = await searchTorrents('', { content_type: contentType }, 'date', 'desc', 1, 0, true)
    if (response.facets) {
      categoryFacets.value = response.facets
    }
  } catch {
    // silent fail
  }
}

function handleSearch(q: string) {
  searchStore.setQuery(q)
}

function handleCategorySelect(value: string | undefined) {
  searchStore.setFilter('content_type', value)
  if (!searchStore.hasActiveSearch && value) {
    searchStore.setQuery('')
    searchStore.setFilter('content_type', value)
  }
  if (value) {
    loadCategoryFacets(value)
  } else {
    categoryFacets.value = null
  }
}

function handleFilterRemove(key: string) {
  searchStore.setFilter(key as keyof typeof searchStore.filters, undefined)
}

function handleSubFilter(key: string, value: string | undefined) {
  searchStore.setFilter(key as keyof typeof searchStore.filters, value)
}

function handleSort(field: string) {
  searchStore.setSort(field)
}

function handleRecentSort(field: string) {
  if (recentSortField.value === field) {
    recentSortOrder.value = recentSortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    recentSortField.value = field
    recentSortOrder.value = 'desc'
  }
}

function setupObserver() {
  if (observer) observer.disconnect()
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && searchStore.hasMore && !searchStore.loading && !searchStore.loadingMore) {
        searchStore.loadMore()
      }
    },
    { rootMargin: '200px' }
  )
  if (sentinel.value) observer.observe(sentinel.value)
}

onMounted(() => {
  searchStore.syncFromURL()
  loadCategories()

  if (searchStore.filters.content_type) {
    loadCategoryFacets(searchStore.filters.content_type)
  }

  if (searchStore.hasActiveSearch) {
    searchStore.search()
  } else {
    loadRecent()
  }
})

onUnmounted(() => {
  if (observer) observer.disconnect()
})

watch(() => searchStore.hasActiveSearch, (isActive) => {
  if (!isActive) {
    loadRecent()
  } else {
    nextTick(setupObserver)
  }
})

watch(sentinel, (el) => {
  if (el && searchStore.hasActiveSearch) setupObserver()
})
</script>

<template>
  <div class="home-view">
    <div class="container">
      <!-- Search Header -->
      <div class="search-header">
        <SearchBar
          :model-value="searchStore.query"
          @update:model-value="searchStore.query = $event"
          @search="handleSearch"
        />
      </div>

      <!-- Active Filters -->
      <ActiveFilters
        v-if="searchStore.hasActiveSearch"
        :filters="searchStore.filters"
        @remove="handleFilterRemove"
        @clear="searchStore.clearFilters()"
      />

      <!-- Advanced Filters -->
      <AdvancedFilters
        v-if="searchStore.hasActiveSearch"
        :facets="searchStore.facets"
        :filters="searchStore.filters"
        @filter="handleSubFilter"
      />

      <div class="home-content">
        <!-- Sidebar -->
        <CategorySidebar
          :categories="categories"
          :selected="searchStore.filters.content_type"
          :facets="categoryFacets"
          @select="handleCategorySelect"
          @filter="handleSubFilter"
        />

        <!-- Main Content -->
        <div class="main-content">
          <!-- Loading -->
          <div v-if="searchStore.loading || recentLoading" class="loading-state">
            <Spinner :size="32" />
          </div>

          <!-- Error -->
          <div v-else-if="searchStore.error" class="error-state card-padded">
            <p>{{ searchStore.error }}</p>
          </div>

          <!-- Search Results -->
          <template v-else-if="searchStore.hasActiveSearch">
            <div class="results-header">
              <h2 class="section-title">
                {{ searchStore.total.toLocaleString() }} results
                <span v-if="searchStore.query">for "{{ searchStore.query }}"</span>
              </h2>
              <div class="results-controls">
                <label class="per-page">
                  <span>Show</span>
                  <select
                    :value="searchStore.limit"
                    @change="searchStore.setLimit(Number(($event.target as HTMLSelectElement).value))"
                    class="per-page-select"
                  >
                    <option v-for="opt in searchStore.limitOptions" :key="opt" :value="opt">{{ opt }}</option>
                  </select>
                  <span>per load</span>
                </label>
              </div>
            </div>
            <div class="card">
              <TorrentTable
                :torrents="searchStore.results"
                :sort-field="searchStore.sort"
                :sort-order="searchStore.order"
                :loading="searchStore.loading"
                @sort="handleSort"
              />
            </div>
            <div class="scroll-status">
              <div v-if="searchStore.loadingMore" class="loading-more">
                <Spinner :size="20" />
                <span>Loading more...</span>
              </div>
              <div v-else-if="!searchStore.hasMore && searchStore.results.length > 0" class="end-of-results">
                Showing all {{ searchStore.total.toLocaleString() }} results
              </div>
            </div>
            <div ref="sentinel" class="scroll-sentinel"></div>
          </template>

          <!-- Recent Torrents -->
          <template v-else>
            <h2 class="section-title">Recently Indexed</h2>
            <div class="card">
              <TorrentTable
                :torrents="sortedRecentTorrents"
                :sort-field="recentSortField"
                :sort-order="recentSortOrder"
                :loading="recentLoading"
                @sort="handleRecentSort"
              />
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-view {
  padding-bottom: 40px;
}

.search-header {
  display: flex;
  justify-content: center;
  padding: 32px 0 16px;
}

.home-content {
  display: flex;
  gap: 24px;
}

.main-content {
  flex: 1;
  min-width: 0;
}

.results-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.results-header .section-title span {
  color: var(--text-secondary);
  font-weight: 400;
}

.results-controls {
  flex-shrink: 0;
}

.per-page {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.per-page-select {
  background: var(--surface);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 4px 8px;
  font-size: 0.8125rem;
  cursor: pointer;
}

.scroll-status {
  padding: 16px 0;
  text-align: center;
}

.loading-more {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-secondary);
  font-size: 0.8125rem;
}

.end-of-results {
  color: var(--text-secondary);
  font-size: 0.8125rem;
}

.scroll-sentinel {
  height: 1px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 60px 0;
}

.error-state {
  text-align: center;
  color: var(--accent);
}

@media (max-width: 640px) {
  .search-header {
    padding: 16px 0 8px;
  }

  .home-content {
    flex-direction: column;
  }
}
</style>
