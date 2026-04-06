<script setup lang="ts">
import { useFormatters } from '@/composables/useFormatters'
import Badge from '@/components/Badge.vue'
import MagnetButton from '@/components/MagnetButton.vue'

interface TorrentRow {
  info_hash: string
  name: string
  size: number
  content_type?: string | null
  resolution?: string | null
  seed_count: number
  peer_count: number
  resolved_at?: string | null
  discovered_at?: string
  source?: string | null
}

const props = defineProps<{
  torrents: TorrentRow[]
  sortField?: string
  sortOrder?: 'asc' | 'desc'
  loading?: boolean
  showDate?: boolean
}>()

const emit = defineEmits<{
  sort: [field: string]
}>()

const { formatBytes, formatDate } = useFormatters()

function getSortIndicator(field: string): string {
  if (props.sortField !== field) return ''
  return props.sortOrder === 'asc' ? '\u25B2' : '\u25BC'
}

function isSortActive(field: string): boolean {
  return props.sortField === field
}

function handleSort(field: string) {
  emit('sort', field)
}
</script>

<template>
  <div class="table-wrapper">
    <table>
      <thead>
        <tr>
          <th class="col-name sortable" @click="handleSort('name')">
            Name
            <span class="sort-indicator" :class="{ active: isSortActive('name') }">{{ getSortIndicator('name') || '\u25BC' }}</span>
          </th>
          <th class="col-type sortable" @click="handleSort('content_type')">
            Type
            <span class="sort-indicator" :class="{ active: isSortActive('content_type') }">{{ getSortIndicator('content_type') || '\u25BC' }}</span>
          </th>
          <th class="col-size sortable" @click="handleSort('size')">
            Size
            <span class="sort-indicator" :class="{ active: isSortActive('size') }">{{ getSortIndicator('size') || '\u25BC' }}</span>
          </th>
          <th class="col-seeders sortable" @click="handleSort('seeders')">
            Seeders
            <span class="sort-indicator" :class="{ active: isSortActive('seeders') }">{{ getSortIndicator('seeders') || '\u25BC' }}</span>
          </th>
          <th class="col-leechers sortable" @click="handleSort('leechers')">
            Leechers
            <span class="sort-indicator" :class="{ active: isSortActive('leechers') }">{{ getSortIndicator('leechers') || '\u25BC' }}</span>
          </th>
          <th class="col-magnet"></th>
          <th class="col-date sortable" v-if="showDate !== false" @click="handleSort('date')">
            Date
            <span class="sort-indicator" :class="{ active: isSortActive('date') }">{{ getSortIndicator('date') || '\u25BC' }}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="loading && torrents.length === 0">
          <td :colspan="showDate !== false ? 7 : 6" class="empty-cell">
            Loading...
          </td>
        </tr>
        <tr v-else-if="torrents.length === 0">
          <td :colspan="showDate !== false ? 7 : 6" class="empty-cell">
            No torrents found
          </td>
        </tr>
        <tr v-for="torrent in torrents" :key="torrent.info_hash">
          <td class="col-name">
            <router-link :to="`/torrent/${torrent.info_hash}`" class="torrent-name">
              {{ torrent.name }}
            </router-link>
            <span v-if="torrent.resolution" class="resolution-tag">{{ torrent.resolution }}</span>
          </td>
          <td class="col-type">
            <Badge :type="torrent.content_type" />
          </td>
          <td class="col-size">{{ formatBytes(torrent.size) }}</td>
          <td class="col-seeders">
            <span class="seeders">{{ torrent.seed_count }}</span>
          </td>
          <td class="col-leechers">
            <span class="leechers">{{ torrent.peer_count }}</span>
          </td>
          <td class="col-magnet">
            <MagnetButton :info-hash="torrent.info_hash" :name="torrent.name" />
          </td>
          <td class="col-date" v-if="showDate !== false">
            {{ formatDate(torrent.resolved_at || torrent.discovered_at) }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrapper {
  overflow-x: auto;
}

.col-name {
  min-width: 300px;
}

.col-type {
  width: 90px;
}

.col-size {
  width: 90px;
  white-space: nowrap;
}

.col-seeders {
  width: 70px;
  white-space: nowrap;
}

.col-leechers {
  width: 70px;
  white-space: nowrap;
}

.col-magnet {
  width: 48px;
}

.col-date {
  width: 100px;
  white-space: nowrap;
  color: var(--text-secondary);
  font-size: 0.8125rem;
}

.torrent-name {
  color: var(--text);
  font-weight: 500;
  word-break: break-word;
}

.torrent-name:hover {
  color: var(--accent);
}

.resolution-tag {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 5px;
  font-size: 0.6875rem;
  font-weight: 600;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  vertical-align: middle;
}

.empty-cell {
  text-align: center;
  padding: 40px 12px !important;
  color: var(--text-secondary);
}
</style>
