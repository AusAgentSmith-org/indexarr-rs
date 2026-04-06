<script setup lang="ts">
import type { TorrentContent } from '@/api'
import Badge from '@/components/Badge.vue'

defineProps<{
  content: TorrentContent | null
}>()

interface InfoItem {
  label: string
  value: string | number | null | undefined
}

function getInfoItems(content: TorrentContent | null): InfoItem[] {
  if (!content) return []
  return [
    { label: 'Title', value: content.title },
    { label: 'Year', value: content.year },
    { label: 'Season', value: content.season ? `S${String(content.season).padStart(2, '0')}` : null },
    { label: 'Episode', value: content.episode ? `E${String(content.episode).padStart(2, '0')}` : null },
    { label: 'Resolution', value: content.resolution },
    { label: 'Codec', value: content.codec },
    { label: 'Source', value: content.video_source },
    { label: 'HDR', value: content.hdr },
    { label: 'Modifier', value: content.modifier },
    { label: 'Audio', value: [content.audio_codec, content.audio_channels].filter(Boolean).join(' ') || null },
    { label: 'Group', value: content.group },
    { label: 'TMDB', value: content.tmdb_id ? `tmdb:${content.tmdb_id}` : null },
    { label: 'IMDB', value: content.imdb_id },
  ].filter(item => item.value != null && item.value !== '')
}
</script>

<template>
  <div class="torrent-info" v-if="content">
    <div class="info-header">
      <Badge :type="content.content_type" />
    </div>
    <div class="info-grid">
      <div
        v-for="item in getInfoItems(content)"
        :key="item.label"
        class="info-item"
      >
        <span class="info-label">{{ item.label }}</span>
        <span class="info-value">{{ item.value }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.torrent-info {
  padding: 20px;
}

.info-header {
  margin-bottom: 16px;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.info-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.info-value {
  font-size: 0.9375rem;
  font-weight: 500;
  color: var(--text);
}
</style>
