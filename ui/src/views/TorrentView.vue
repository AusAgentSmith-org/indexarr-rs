<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { getTorrent } from '@/api'
import type { TorrentDetail } from '@/api'
import { useFormatters } from '@/composables/useFormatters'
import TorrentInfo from '@/components/TorrentInfo.vue'
import FileTree from '@/components/FileTree.vue'
import NFOViewer from '@/components/NFOViewer.vue'
import CopyText from '@/components/CopyText.vue'
import Spinner from '@/components/Spinner.vue'
import VoteButtons from '@/components/VoteButtons.vue'
import NukeButton from '@/components/NukeButton.vue'
import CommentSection from '@/components/CommentSection.vue'

const route = useRoute()
const { formatBytes, formatDate, formatNumber } = useFormatters()

const torrent = ref<TorrentDetail | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)

async function load() {
  loading.value = true
  error.value = null
  try {
    const hash = route.params.hash as string
    torrent.value = await getTorrent(hash)
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load torrent'
  } finally {
    loading.value = false
  }
}

function copyMagnet() {
  if (!torrent.value) return
  const uri = torrent.value.magnet_uri ||
    `magnet:?xt=urn:btih:${torrent.value.info_hash}&dn=${encodeURIComponent(torrent.value.name)}&tr=udp://tracker.opentrackr.org:1337/announce`
  navigator.clipboard.writeText(uri)
}

onMounted(load)
</script>

<template>
  <div class="torrent-view">
    <div class="container">
      <!-- Loading -->
      <div v-if="loading" class="loading-state">
        <Spinner :size="32" />
      </div>

      <!-- Error -->
      <div v-else-if="error" class="error-state card-padded">
        <p>{{ error }}</p>
        <router-link to="/" class="btn btn-ghost" style="margin-top: 12px">Back to search</router-link>
      </div>

      <!-- Content -->
      <template v-else-if="torrent">
        <!-- Back link -->
        <div class="back-link">
          <router-link to="/">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="15 18 9 12 15 6" />
            </svg>
            Back to search
          </router-link>
        </div>

        <!-- Header -->
        <div class="torrent-header card-padded">
          <div class="header-top">
            <h1 class="torrent-name">{{ torrent.name }}</h1>
            <div class="header-actions">
              <VoteButtons v-if="torrent.resolved_at" :info-hash="torrent.info_hash" />
              <NukeButton :info-hash="torrent.info_hash" />
              <button class="btn btn-accent" @click="copyMagnet" title="Copy magnet link">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M15 7h3a5 5 0 0 1 5 5 5 5 0 0 1-5 5h-3m-6 0H6a5 5 0 0 1-5-5 5 5 0 0 1 5-5h3" />
                  <line x1="8" y1="12" x2="16" y2="12" />
                </svg>
                Magnet
              </button>
            </div>
          </div>
        </div>

        <!-- Info Grid -->
        <div class="detail-grid">
          <!-- Parsed Content Info -->
          <div class="card" v-if="torrent.content">
            <div class="card-header">
              <h3>Content Info</h3>
            </div>
            <TorrentInfo :content="torrent.content" />
          </div>

          <!-- Metadata -->
          <div class="card">
            <div class="card-header">
              <h3>Metadata</h3>
            </div>
            <div class="meta-item-full">
              <span class="meta-label">Info Hash</span>
              <CopyText :text="torrent.info_hash" />
            </div>
            <div class="metadata-grid">
              <div class="meta-item">
                <span class="meta-label">Size</span>
                <span class="meta-value">{{ formatBytes(torrent.size) }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Source</span>
                <span class="meta-value">{{ torrent.source || '-' }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Discovered</span>
                <span class="meta-value">{{ formatDate(torrent.discovered_at) }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Resolved</span>
                <span class="meta-value">{{ formatDate(torrent.resolved_at) }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Seeders</span>
                <span class="meta-value seeders">{{ torrent.seed_count }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Leechers</span>
                <span class="meta-value leechers">{{ torrent.peer_count }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Observations</span>
                <span class="meta-value">{{ formatNumber(torrent.observations) }}</span>
              </div>
              <div class="meta-item">
                <span class="meta-label">Private</span>
                <span class="meta-value">{{ torrent.private ? 'Yes' : 'No' }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Tags -->
        <div class="card-padded" v-if="torrent.tags && torrent.tags.length > 0">
          <h3 class="card-section-title">Tags</h3>
          <div class="tag-list">
            <span v-for="tag in torrent.tags" :key="tag" class="tag-badge">
              {{ tag }}
            </span>
          </div>
        </div>

        <!-- Files -->
        <div class="card" v-if="torrent.files && torrent.files.length > 0">
          <div class="card-header">
            <h3>Files ({{ torrent.files.length }})</h3>
          </div>
          <div class="files-content">
            <FileTree :files="torrent.files" />
          </div>
        </div>

        <!-- NFO -->
        <div v-if="torrent.nfo" class="nfo-section">
          <h3 class="section-title">NFO</h3>
          <NFOViewer :nfo="torrent.nfo" />
        </div>

        <!-- Comments -->
        <CommentSection v-if="torrent.resolved_at" :info-hash="torrent.info_hash" />
      </template>
    </div>
  </div>
</template>

<style scoped>
.torrent-view {
  padding-bottom: 40px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 80px 0;
}

.error-state {
  text-align: center;
  color: var(--accent);
  margin-top: 40px;
}

.back-link {
  padding: 16px 0;
}

.back-link a {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.back-link a:hover {
  color: var(--text);
  text-decoration: none;
}

.torrent-header {
  margin-bottom: 16px;
}

.header-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.torrent-name {
  font-size: 1.25rem;
  font-weight: 600;
  line-height: 1.4;
  word-break: break-word;
  flex: 1;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 16px;
}

.card-header {
  padding: 16px 20px 0;
}

.card-header h3 {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.meta-item-full {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 16px 20px 0;
  overflow: hidden;
}

.meta-item-full :deep(code) {
  word-break: break-all;
}

.metadata-grid {
  padding: 16px 20px 20px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.meta-value {
  font-size: 0.9375rem;
  font-weight: 500;
}

.card-section-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag-badge {
  display: inline-flex;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 500;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text);
}

.files-content {
  padding: 12px 16px 16px;
}

.nfo-section {
  margin-top: 16px;
}

@media (max-width: 768px) {
  .detail-grid {
    grid-template-columns: 1fr;
  }

  .header-top {
    flex-direction: column;
  }
}
</style>
