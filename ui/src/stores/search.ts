import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { searchTorrents } from '@/api'
import type { TorrentResult, Facets, SearchFilters } from '@/api'

const LIMIT_OPTIONS = [25, 50, 100] as const
const LIMIT_STORAGE_KEY = 'indexarr_search_limit'

function loadSavedLimit(): number {
  try {
    const saved = localStorage.getItem(LIMIT_STORAGE_KEY)
    if (saved) {
      const n = parseInt(saved, 10)
      if ((LIMIT_OPTIONS as readonly number[]).includes(n)) return n
    }
  } catch { /* ignore */ }
  return 50
}

export const useSearchStore = defineStore('search', () => {
  const router = useRouter()
  const route = useRoute()

  const query = ref('')
  const filters = ref<SearchFilters>({})
  const results = ref<TorrentResult[]>([])
  const facets = ref<Facets | null>(null)
  const loading = ref(false)
  const loadingMore = ref(false)
  const total = ref(0)
  const offset = ref(0)
  const limit = ref(loadSavedLimit())
  const sort = ref('date')
  const order = ref<'asc' | 'desc'>('desc')
  const error = ref<string | null>(null)

  const hasActiveSearch = computed(() => {
    return query.value.trim() !== '' || Object.values(filters.value).some(v => !!v)
  })

  const hasMore = computed(() => results.value.length < total.value)

  const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
  const totalPages = computed(() => Math.ceil(total.value / limit.value))

  function syncFromURL() {
    if (route.name !== 'search') return
    const q = route.query
    query.value = (q.q as string) || ''
    filters.value = {
      content_type: (q.content_type as string) || undefined,
      resolution: (q.resolution as string) || undefined,
      codec: (q.codec as string) || undefined,
      video_source: (q.video_source as string) || undefined,
      hdr: (q.hdr as string) || undefined,
      year: (q.year as string) || undefined,
      source: (q.source as string) || undefined,
      audio_codec: (q.audio_codec as string) || undefined,
      modifier: (q.modifier as string) || undefined,
      platform: (q.platform as string) || undefined,
      has_subtitles: (q.has_subtitles as string) || undefined,
      music_format: (q.music_format as string) || undefined,
      category: (q.category as string) || undefined,
      language: (q.language as string) || undefined,
    }
    sort.value = (q.sort as string) || 'date'
    order.value = ((q.order as string) || 'desc') as 'asc' | 'desc'
  }

  function syncToURL() {
    if (route.name !== 'search') return
    const queryParams: Record<string, string> = {}
    if (query.value) queryParams.q = query.value
    for (const [key, value] of Object.entries(filters.value)) {
      if (value) queryParams[key] = value
    }
    if (sort.value !== 'date') queryParams.sort = sort.value
    if (order.value !== 'desc') queryParams.order = order.value

    router.replace({ query: queryParams })
  }

  async function search() {
    if (!hasActiveSearch.value) return

    loading.value = true
    error.value = null
    offset.value = 0
    try {
      const response = await searchTorrents(
        query.value,
        filters.value,
        sort.value,
        order.value,
        limit.value,
        0
      )
      results.value = response.results
      total.value = response.total
      if (response.facets) {
        facets.value = response.facets
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Search failed'
      results.value = []
      total.value = 0
    } finally {
      loading.value = false
    }
  }

  async function loadMore() {
    if (!hasActiveSearch.value || loadingMore.value || !hasMore.value) return

    loadingMore.value = true
    const nextOffset = results.value.length
    try {
      const response = await searchTorrents(
        query.value,
        filters.value,
        sort.value,
        order.value,
        limit.value,
        nextOffset
      )
      results.value = [...results.value, ...response.results]
      total.value = response.total
    } catch {
      // silent — user can scroll again to retry
    } finally {
      loadingMore.value = false
    }
  }

  function setQuery(q: string) {
    query.value = q
    offset.value = 0
    syncToURL()
    search()
  }

  function setFilter(key: keyof SearchFilters, value: string | undefined) {
    if (value) {
      filters.value = { ...filters.value, [key]: value }
    } else {
      const newFilters = { ...filters.value }
      delete newFilters[key]
      filters.value = newFilters
    }
    offset.value = 0
    syncToURL()
    search()
  }

  function clearFilters() {
    filters.value = {}
    offset.value = 0
    syncToURL()
    if (query.value) {
      search()
    }
  }

  function setSort(field: string) {
    if (sort.value === field) {
      order.value = order.value === 'asc' ? 'desc' : 'asc'
    } else {
      sort.value = field
      order.value = 'desc'
    }
    offset.value = 0
    syncToURL()
    if (hasActiveSearch.value) {
      search()
    }
  }

  function setPage(page: number) {
    offset.value = (page - 1) * limit.value
    syncToURL()
    search()
  }

  function setLimit(newLimit: number) {
    limit.value = newLimit
    try { localStorage.setItem(LIMIT_STORAGE_KEY, String(newLimit)) } catch { /* ignore */ }
    offset.value = 0
    syncToURL()
    if (hasActiveSearch.value) {
      search()
    }
  }

  return {
    query,
    filters,
    results,
    facets,
    loading,
    loadingMore,
    total,
    offset,
    limit,
    sort,
    order,
    error,
    hasActiveSearch,
    hasMore,
    currentPage,
    totalPages,
    limitOptions: LIMIT_OPTIONS,
    syncFromURL,
    syncToURL,
    search,
    loadMore,
    setQuery,
    setFilter,
    clearFilters,
    setSort,
    setPage,
    setLimit,
  }
})
