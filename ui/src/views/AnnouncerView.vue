<script setup lang="ts">
import { ref, computed } from 'vue'
import { getAnnouncerStatus, getRecentlyAnnounced } from '@/api'
import type { AnnouncerStatus, AnnouncedResult } from '@/api'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { useFormatters } from '@/composables/useFormatters'
import Spinner from '@/components/Spinner.vue'
import StatCard from '@/components/StatCard.vue'
import TorrentTable from '@/components/TorrentTable.vue'

const { formatNumber } = useFormatters()

const status = ref<AnnouncerStatus | null>(null)
const recentAnnounced = ref<AnnouncedResult[]>([])
const error = ref<string | null>(null)

async function loadStatus() {
  try {
    const [s, r] = await Promise.all([
      getAnnouncerStatus(),
      getRecentlyAnnounced(50),
    ])
    status.value = s
    recentAnnounced.value = r.results
    error.value = null
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load announcer status'
  }
}

const recentAsRows = computed(() =>
  recentAnnounced.value.map(r => ({
    ...r,
    resolved_at: r.announced_at,
  }))
)

const { isRefreshing } = useAutoRefresh(loadStatus, 10000)

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${Math.floor(seconds)}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h}h ${m}m`
}

const announceProgress = computed(() => {
  if (!status.value) return 0
  const total = status.value.announced_count + status.value.pending_announce_count
  if (total === 0) return 0
  return (status.value.announced_count / total) * 100
})

const revalidationProgress = computed(() => {
  if (!status.value || status.value.announced_count === 0) return 0
  return (status.value.validated_7d / status.value.announced_count) * 100
})

const oldestValidationAge = computed(() => {
  if (!status.value?.oldest_validation) return null
  const oldest = new Date(status.value.oldest_validation)
  const now = new Date()
  const diffMs = now.getTime() - oldest.getTime()
  const diffH = Math.floor(diffMs / 3600000)
  if (diffH < 1) return 'less than 1 hour'
  if (diffH < 24) return `${diffH}h ago`
  const diffD = Math.floor(diffH / 24)
  return `${diffD}d ago`
})

const throughput = computed(() => {
  if (!status.value || status.value.uptime_seconds < 60) return null
  const hours = status.value.uptime_seconds / 3600
  if (hours === 0) return null
  return Math.round(status.value.total_announced_session / hours)
})
</script>

<template>
  <div class="announcer-view">
    <div class="container">
      <div class="page-header">
        <h1 class="page-title">Announcer</h1>
        <div class="refresh-indicator" v-if="isRefreshing">
          <Spinner :size="16" />
        </div>
      </div>

      <!-- Error -->
      <div v-if="error && !status" class="error-state card-padded">
        <p>{{ error }}</p>
      </div>

      <!-- Loading -->
      <div v-else-if="!status" class="loading-state">
        <Spinner :size="32" />
      </div>

      <template v-else>
        <!-- Worker Not Active Banner -->
        <div v-if="!status.running" class="inactive-banner card-padded">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" /><line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
          </svg>
          <div>
            <strong>Announcer not active on this node</strong>
            <p>This worker is not included in the current node's worker configuration. Database statistics are still shown below.</p>
          </div>
        </div>

        <!-- Stat Cards -->
        <div class="grid-4 stats-grid" :class="{ 'inactive-section': !status.running }">
          <StatCard
            label="Announced"
            :value="status.announced_count"
            icon="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"
            color="#2ecc71"
          />
          <StatCard
            label="Pending"
            :value="status.pending_announce_count"
            icon="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2"
            color="#e67e22"
          />
          <StatCard
            label="Pool Active"
            :value="status.pool_active"
            icon="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
            color="var(--accent)"
          />
          <StatCard
            label="Session Total"
            :value="status.total_announced_session"
            icon="M9 11l3 3L22 4M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"
            color="var(--info)"
          />
        </div>

        <!-- New Entries Progress -->
        <div class="card-padded progress-section" :class="{ 'inactive-section': !status.running }">
          <div class="progress-header">
            <h3 class="card-title">New Entries</h3>
            <span class="progress-pct">{{ announceProgress.toFixed(1) }}%</span>
          </div>
          <div class="progress-track">
            <div class="progress-fill new" :style="{ width: announceProgress + '%' }"></div>
          </div>
          <div class="progress-labels">
            <span>{{ formatNumber(status.announced_count) }} announced</span>
            <span>{{ formatNumber(status.pending_announce_count) }} pending</span>
          </div>
        </div>

        <!-- Revalidation Freshness -->
        <div class="card-padded progress-section" :class="{ 'inactive-section': !status.running }">
          <div class="progress-header">
            <h3 class="card-title">Revalidation Freshness</h3>
            <span class="progress-pct">{{ revalidationProgress.toFixed(1) }}%</span>
          </div>
          <div class="progress-track">
            <div class="progress-fill reval" :style="{ width: revalidationProgress + '%' }"></div>
          </div>
          <div class="progress-labels">
            <span>{{ formatNumber(status.validated_24h) }} last 24h &middot; {{ formatNumber(status.validated_7d) }} last 7d</span>
            <span v-if="oldestValidationAge">Oldest: {{ oldestValidationAge }}</span>
          </div>
        </div>

        <!-- Status Cards -->
        <div class="status-grid" :class="{ 'inactive-section': !status.running }">
          <div class="card-padded">
            <h3 class="card-title">Worker Status</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Status</span>
                <span class="status-value">
                  <span class="status-dot" :class="status.running ? 'running' : 'stopped'"></span>
                  {{ status.running ? 'Running' : 'Not Active' }}
                </span>
              </div>
              <div class="status-row">
                <span class="status-label">Enabled</span>
                <span class="status-value">{{ status.enabled ? 'Yes' : 'No' }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Uptime</span>
                <span class="status-value">{{ formatUptime(status.uptime_seconds) }}</span>
              </div>
              <div class="status-row" v-if="throughput !== null">
                <span class="status-label">Throughput</span>
                <span class="status-value">~{{ formatNumber(throughput) }}/hr</span>
              </div>
            </div>
          </div>

          <div class="card-padded">
            <h3 class="card-title">Configuration</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Pool Size</span>
                <span class="status-value">{{ status.pool_active }} / {{ status.pool_size }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Settled</span>
                <span class="status-value">{{ status.pool_settled }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Settle Time</span>
                <span class="status-value">{{ status.settle_time }}s</span>
              </div>
              <div class="status-row">
                <span class="status-label">Rotate Interval</span>
                <span class="status-value">{{ status.rotate_interval }}s</span>
              </div>
              <div class="status-row">
                <span class="status-label">Listen Port</span>
                <span class="status-value">{{ status.port }}</span>
              </div>
            </div>
          </div>

          <div class="card-padded">
            <h3 class="card-title">Features</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">NFO Download</span>
                <span class="status-value">
                  <span class="feature-badge" :class="status.download_nfo ? 'on' : 'off'">
                    {{ status.download_nfo ? 'Enabled' : 'Disabled' }}
                  </span>
                </span>
              </div>
              <div class="status-row">
                <span class="status-label">VPN Proxy</span>
                <span class="status-value">
                  <span class="feature-badge" :class="status.proxy_configured ? 'on' : 'off'">
                    {{ status.proxy_configured ? 'Configured' : 'None' }}
                  </span>
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Info Banner -->
        <div class="info-banner card-padded">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          <p>
            The announcer validates torrents by contacting their trackers and DHT peers via libtorrent.
            It maintains a rolling pool of up to {{ status.pool_size }} torrents, continuously adding new ones
            as completed ones are harvested. Each torrent settles for {{ status.settle_time }}s then is harvested
            after {{ status.rotate_interval }}s. New torrents always take priority; once caught up, the announcer
            continuously revalidates the stalest entries to keep seed/peer counts fresh.
          </p>
        </div>

        <!-- Recently Announced Feed -->
        <div class="recent-section">
          <h2 class="section-title">Recently Announced</h2>
          <div class="card-padded">
            <TorrentTable
              :torrents="recentAsRows"
              :show-date="true"
              :loading="!status"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.announcer-view {
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
  margin-bottom: 24px;
}

/* Progress section */
.progress-section {
  margin-bottom: 24px;
}

.progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.progress-pct {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text);
}

.progress-track {
  height: 10px;
  background: var(--surface);
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: 8px;
}

.progress-fill {
  height: 100%;
  border-radius: 5px;
  transition: width 0.5s ease;
}

.progress-fill.new {
  background: linear-gradient(90deg, #2ecc71, #27ae60);
}

.progress-fill.reval {
  background: linear-gradient(90deg, #3498db, #2980b9);
}

.progress-labels {
  display: flex;
  justify-content: space-between;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

/* Status grid */
.status-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.card-title {
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin-bottom: 16px;
}

.status-items {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.status-label {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.status-value {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot.running {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.status-dot.stopped {
  background: var(--text-secondary);
}

.feature-badge {
  padding: 2px 8px;
  border-radius: var(--radius);
  font-size: 0.75rem;
  font-weight: 600;
}

.feature-badge.on {
  background: rgba(46, 204, 113, 0.15);
  color: #2ecc71;
}

.feature-badge.off {
  background: var(--surface);
  color: var(--text-secondary);
}

/* Info banner */
.info-banner {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  color: var(--text-secondary);
  font-size: 0.8125rem;
  line-height: 1.5;
}

.info-banner svg {
  flex-shrink: 0;
  margin-top: 1px;
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

.inactive-banner {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 20px;
  border-left: 3px solid var(--text-secondary);
  color: var(--text-secondary);
}

.inactive-banner strong {
  color: var(--text);
  display: block;
  margin-bottom: 4px;
}

.inactive-banner p {
  font-size: 0.8125rem;
  line-height: 1.4;
  margin: 0;
}

.inactive-banner svg {
  flex-shrink: 0;
  margin-top: 2px;
  opacity: 0.6;
}

.inactive-section {
  opacity: 0.5;
}

/* Recently announced feed */
.recent-section {
  margin-top: 32px;
}

.section-title {
  font-size: 1.125rem;
  font-weight: 700;
  margin-bottom: 16px;
}

@media (max-width: 768px) {
  .status-grid {
    grid-template-columns: 1fr;
  }
}
</style>
