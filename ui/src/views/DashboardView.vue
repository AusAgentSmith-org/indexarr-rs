<script setup lang="ts">
import { ref } from 'vue'
import { getStats, getDHTStatus, getSyncDashboard, getTrending, getRecentComments } from '@/api'
import type { Stats, DHTStatus, SyncDashboard, SyncPeer, TrendingTorrent, RecentComment } from '@/api'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { useFormatters } from '@/composables/useFormatters'
import StatCard from '@/components/StatCard.vue'
import BarChart from '@/components/BarChart.vue'
import Spinner from '@/components/Spinner.vue'

const { formatBytes, formatDate, formatNumber } = useFormatters()

const stats = ref<Stats | null>(null)
const dht = ref<DHTStatus | null>(null)
const sync = ref<SyncDashboard | null>(null)
const trending = ref<TrendingTorrent[]>([])
const comments = ref<RecentComment[]>([])
const error = ref<string | null>(null)
const bootstrapDismissed = ref(false)

const bootstrapSteps = [
  { key: 'connecting', label: 'Connecting', description: 'Finding peers on the network' },
  { key: 'downloading', label: 'Downloading', description: 'Downloading data from peers' },
  { key: 'importing', label: 'Importing', description: 'Merging records into database' },
  { key: 'complete', label: 'Complete', description: 'Initial sync finished' },
] as const

function bootstrapStepStatus(stepKey: string, phase: string): 'done' | 'active' | 'pending' {
  const order = ['connecting', 'downloading', 'importing', 'complete']
  const stepIdx = order.indexOf(stepKey)
  const phaseIdx = order.indexOf(phase)
  if (stepIdx < phaseIdx) return 'done'
  if (stepIdx === phaseIdx) return 'active'
  return 'pending'
}

const contentTypeColors: Record<string, string> = {
  movie: '#3498db',
  tv_show: '#2ecc71',
  music: '#9b59b6',
  game: '#e67e22',
  software: '#1abc9c',
  ebook: '#e74c3c',
  xxx: '#95a5a6',
  unknown: '#bdc3c7',
}

const sourceColors: Record<string, string> = {
  dht: '#3498db',
  api: '#2ecc71',
  sync: '#9b59b6',
  manual: '#e67e22',
}

async function loadAll() {
  try {
    const [statsRes, dhtRes, syncRes, trendRes, commRes] = await Promise.allSettled([
      getStats(),
      getDHTStatus(),
      getSyncDashboard(),
      getTrending(12, 10),
      getRecentComments(15),
    ])
    if (statsRes.status === 'fulfilled') stats.value = statsRes.value
    if (dhtRes.status === 'fulfilled') dht.value = dhtRes.value
    if (syncRes.status === 'fulfilled') sync.value = syncRes.value
    if (trendRes.status === 'fulfilled') trending.value = trendRes.value.results
    if (commRes.status === 'fulfilled') comments.value = commRes.value.comments
    error.value = null
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load dashboard'
  }
}

const { isRefreshing } = useAutoRefresh(loadAll, 30000)

function contentTypeItems() {
  if (!stats.value) return []
  return stats.value.by_content_type.map(ct => ({
    label: ct.content_type || 'unknown',
    value: ct.count,
    color: contentTypeColors[ct.content_type] || '#bdc3c7',
  }))
}

function sourceItems() {
  if (!stats.value) return []
  return stats.value.by_source.map(s => ({
    label: s.source || 'unknown',
    value: s.count,
    color: sourceColors[s.source] || '#6c757d',
  }))
}

function contentTypeLabel(ct: string | null): string {
  if (!ct) return 'Unknown'
  return ct.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
}
</script>

<template>
  <div class="dashboard-view">
    <div class="container">
      <div class="page-header">
        <h1 class="page-title">Dashboard</h1>
        <div class="refresh-indicator" v-if="isRefreshing">
          <Spinner :size="16" />
        </div>
      </div>

      <!-- Error -->
      <div v-if="error" class="error-state card-padded">
        <p>{{ error }}</p>
      </div>

      <!-- Loading -->
      <div v-else-if="!stats" class="loading-state">
        <Spinner :size="32" />
      </div>

      <template v-else>
        <!-- ============================================================ -->
        <!-- SERVICE STATUS                                                -->
        <!-- ============================================================ -->

        <div class="grid-4 stats-grid">
          <StatCard
            label="Index Size"
            :value="stats.resolved"
            icon="M22 11.08V12a10 10 0 1 1-5.93-9.14M22 4L12 14.01l-3-3"
            color="var(--success)"
          />
          <StatCard
            label="Pending Resolution"
            :value="stats.pending"
            icon="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2"
            color="var(--warning)"
          />
          <StatCard
            label="Dead Hashes"
            :value="stats.dead"
            icon="M18 6L6 18M6 6l12 12"
            color="var(--text-secondary)"
          />
          <StatCard
            label="Total Hashes Seen"
            :value="stats.total"
            icon="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
            color="var(--accent)"
          />
        </div>

        <!-- Service Indicators -->
        <div class="service-row">
          <div class="service-card card-padded">
            <div class="service-header">
              <span class="service-dot" :class="dht?.dht_running ? 'dot-ok' : 'dot-off'"></span>
              <span class="service-name">DHT Crawler</span>
            </div>
            <template v-if="dht">
              <div class="service-stats">
                <span>{{ dht.instances }} instances</span>
                <span>{{ formatNumber(dht.routing_table_good) }} good nodes</span>
                <span>{{ formatNumber(dht.hash_queue_size) }} queued</span>
              </div>
            </template>
            <span v-else class="service-unavailable">Not running</span>
          </div>

          <div class="service-card card-padded">
            <div class="service-header">
              <span class="service-dot" :class="sync?.enabled ? 'dot-ok' : 'dot-off'"></span>
              <span class="service-name">Sync</span>
            </div>
            <template v-if="sync?.enabled">
              <div class="service-stats">
                <span>{{ sync.peers.length }}/{{ sync.max_peers }} peers</span>
                <span>{{ sync.peers.filter((p: SyncPeer) => p.healthy).length }} healthy</span>
                <span>{{ formatNumber(sync.total_imported) }} imported</span>
              </div>
            </template>
            <span v-else class="service-unavailable">Not enabled</span>
          </div>
        </div>

        <!-- ============================================================ -->
        <!-- INITIAL SYNC (Bootstrap)                                      -->
        <!-- ============================================================ -->

        <div
          v-if="sync?.bootstrap && !bootstrapDismissed"
          class="card-padded bootstrap-panel"
        >
          <div class="bootstrap-header">
            <h2 class="section-title">Initial Sync</h2>
            <button
              v-if="sync.bootstrap.phase === 'complete'"
              class="btn-dismiss"
              @click="bootstrapDismissed = true"
            >Dismiss</button>
          </div>
          <div class="bootstrap-steps">
            <div
              v-for="bstep in bootstrapSteps"
              :key="bstep.key"
              class="bootstrap-step"
              :class="'step-' + bootstrapStepStatus(bstep.key, sync.bootstrap.phase)"
            >
              <div class="step-indicator">
                <span class="step-dot"></span>
                <span v-if="bstep.key !== 'complete'" class="step-line"></span>
              </div>
              <div class="step-content">
                <div class="step-label">{{ bstep.label }}</div>
                <div class="step-detail">
                  <template v-if="bstep.key === 'connecting'">
                    {{ sync.bootstrap.peers_found }} peer(s) found
                  </template>
                  <template v-else-if="bstep.key === 'downloading'">
                    {{ sync.bootstrap.deltas_downloaded }} / {{ sync.bootstrap.deltas_total || '?' }} deltas
                  </template>
                  <template v-else-if="bstep.key === 'importing'">
                    {{ sync.bootstrap.records_imported.toLocaleString() }}
                    <template v-if="sync.bootstrap.records_total > 0">
                      / {{ sync.bootstrap.records_total.toLocaleString() }}
                    </template>
                    records
                  </template>
                  <template v-else-if="bstep.key === 'complete'">
                    {{ bstep.description }}
                  </template>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ============================================================ -->
        <!-- TRENDING + COMMENTS                                           -->
        <!-- ============================================================ -->

        <div class="two-col">
          <!-- Trending Torrents -->
          <div class="card-padded">
            <h2 class="section-title">Trending <span>last 12 hours</span></h2>
            <div v-if="trending.length === 0" class="empty-state">
              No torrents resolved in the last 12 hours
            </div>
            <div v-else class="trending-list">
              <router-link
                v-for="(t, i) in trending"
                :key="t.info_hash"
                :to="`/torrent/${t.info_hash}`"
                class="trending-item"
              >
                <span class="trending-rank">{{ i + 1 }}</span>
                <div class="trending-info">
                  <span class="trending-name">{{ t.name }}</span>
                  <div class="trending-meta">
                    <span v-if="t.content_type" class="trending-type">{{ contentTypeLabel(t.content_type) }}</span>
                    <span class="trending-size">{{ formatBytes(t.size) }}</span>
                  </div>
                </div>
                <div class="trending-seeds">
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" />
                  </svg>
                  <span>{{ formatNumber(t.seed_count) }}</span>
                </div>
              </router-link>
            </div>
          </div>

          <!-- Recent Comments -->
          <div class="card-padded">
            <h2 class="section-title">Recent Comments</h2>
            <div v-if="comments.length === 0" class="empty-state">
              No comments yet
            </div>
            <div v-else class="comment-feed">
              <div v-for="c in comments" :key="c.id" class="comment-item">
                <div class="comment-header">
                  <span class="comment-nick">{{ c.nickname }}</span>
                  <span class="comment-time">{{ formatDate(c.created_at) }}</span>
                </div>
                <p class="comment-body">{{ c.body }}</p>
                <router-link :to="`/torrent/${c.info_hash}`" class="comment-torrent">
                  {{ c.torrent_name || c.info_hash.slice(0, 12) + '...' }}
                </router-link>
              </div>
            </div>
          </div>
        </div>

        <!-- ============================================================ -->
        <!-- CHARTS                                                        -->
        <!-- ============================================================ -->

        <div class="charts-grid">
          <div class="card-padded" v-if="contentTypeItems().length > 0">
            <BarChart
              title="Content Types"
              :items="contentTypeItems()"
            />
          </div>
          <div class="card-padded" v-if="sourceItems().length > 0">
            <BarChart
              title="Sources"
              :items="sourceItems()"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.dashboard-view {
  padding-bottom: 40px;
}

.page-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 24px 0 20px;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 700;
}

.refresh-indicator {
  display: flex;
  align-items: center;
}

.stats-grid {
  margin-bottom: 16px;
}

/* ── Service Row ─────────────────────────────────────── */
.service-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 24px;
}

.service-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.service-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-ok {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.dot-off {
  background: var(--text-secondary);
  opacity: 0.4;
}

.service-name {
  font-weight: 600;
  font-size: 0.9375rem;
}

.service-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.service-unavailable {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  font-style: italic;
}

/* ── Two-column layout ───────────────────────────────── */
.two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 24px;
}

.section-title {
  font-size: 1rem;
  font-weight: 700;
  margin-bottom: 16px;
}

.section-title span {
  font-weight: 400;
  color: var(--text-secondary);
  font-size: 0.8125rem;
}

.empty-state {
  color: var(--text-secondary);
  font-size: 0.875rem;
  text-align: center;
  padding: 24px 0;
}

/* ── Trending List ───────────────────────────────────── */
.trending-list {
  display: flex;
  flex-direction: column;
}

.trending-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
  text-decoration: none;
  color: var(--text);
  transition: background 0.1s;
}

.trending-item:last-child {
  border-bottom: none;
}

.trending-item:hover {
  background: var(--surface);
  margin: 0 -16px;
  padding-left: 16px;
  padding-right: 16px;
}

.trending-rank {
  width: 24px;
  text-align: center;
  font-weight: 700;
  font-size: 0.875rem;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.trending-info {
  flex: 1;
  min-width: 0;
}

.trending-name {
  display: block;
  font-size: 0.8125rem;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.trending-meta {
  display: flex;
  gap: 8px;
  margin-top: 2px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.trending-type {
  text-transform: capitalize;
}

.trending-seeds {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--success);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

/* ── Comment Feed ────────────────────────────────────── */
.comment-feed {
  display: flex;
  flex-direction: column;
  gap: 0;
  max-height: 500px;
  overflow-y: auto;
}

.comment-item {
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.comment-item:last-child {
  border-bottom: none;
}

.comment-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.comment-nick {
  font-weight: 600;
  font-size: 0.8125rem;
}

.comment-time {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.comment-body {
  font-size: 0.8125rem;
  color: var(--text);
  line-height: 1.5;
  margin-bottom: 4px;
  word-break: break-word;
}

.comment-torrent {
  font-size: 0.75rem;
  color: var(--accent);
  text-decoration: none;
  display: inline-block;
  max-width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.comment-torrent:hover {
  text-decoration: underline;
}

/* ── Charts ──────────────────────────────────────────── */
.charts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 80px 0;
}

.error-state {
  text-align: center;
  color: var(--accent);
}

/* ── Bootstrap Panel ─────────────────────────────────── */
.bootstrap-panel {
  border: 1px solid rgba(23, 162, 184, 0.3);
  background: rgba(23, 162, 184, 0.03);
  margin-bottom: 24px;
}

.bootstrap-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.btn-dismiss {
  padding: 4px 12px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-dismiss:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text);
}

.bootstrap-steps {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.bootstrap-step {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}

.step-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
  width: 16px;
}

.step-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}

.step-line {
  width: 2px;
  height: 24px;
  background: var(--border);
}

.step-content {
  padding-bottom: 16px;
}

.step-label {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.step-detail {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

.step-done .step-dot {
  background: var(--success);
  border-color: var(--success);
}

.step-done .step-line {
  background: var(--success);
}

.step-done .step-label {
  color: var(--success);
}

.step-active .step-dot {
  background: #17a2b8;
  border-color: #17a2b8;
  animation: pulse-dot 1.5s ease-in-out infinite;
}

.step-active .step-label {
  color: var(--text);
}

.step-active .step-detail {
  color: var(--text);
}

@keyframes pulse-dot {
  0%, 100% { box-shadow: 0 0 0 0 rgba(23, 162, 184, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(23, 162, 184, 0); }
}

@media (max-width: 768px) {
  .service-row,
  .two-col,
  .charts-grid {
    grid-template-columns: 1fr;
  }
}
</style>
