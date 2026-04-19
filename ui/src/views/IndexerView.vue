<script setup lang="ts">
import { ref, computed } from 'vue'
import { getDHTStatus, getQueue } from '@/api'
import type { DHTStatus, QueueItem } from '@/api'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { useFormatters } from '@/composables/useFormatters'
import Spinner from '@/components/Spinner.vue'
import CopyText from '@/components/CopyText.vue'
import Pagination from '@/components/Pagination.vue'

const { formatDate, formatNumber } = useFormatters()

const dhtStatus = ref<DHTStatus | null>(null)
const queueItems = ref<QueueItem[]>([])
const queueTotal = ref(0)
const queueOffset = ref(0)
const queueLimit = ref(50)
const error = ref<string | null>(null)

const queueCurrentPage = computed(() => Math.floor(queueOffset.value / queueLimit.value) + 1)
const queueTotalPages = computed(() => Math.ceil(queueTotal.value / queueLimit.value))

async function loadAll() {
  try {
    const [status, queue] = await Promise.all([
      getDHTStatus(),
      getQueue(queueLimit.value, queueOffset.value),
    ])
    dhtStatus.value = status
    queueItems.value = queue.results
    queueTotal.value = queue.total
    error.value = null
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load indexer status'
  }
}

const { isRefreshing } = useAutoRefresh(loadAll, 10000)

async function setQueuePage(page: number) {
  queueOffset.value = (page - 1) * queueLimit.value
  try {
    const queue = await getQueue(queueLimit.value, queueOffset.value)
    queueItems.value = queue.results
    queueTotal.value = queue.total
  } catch {
    // silent
  }
}
</script>

<template>
  <div class="indexer-view">
    <div class="container">
      <div class="page-header">
        <h1 class="page-title">DHT Indexer</h1>
        <div class="refresh-indicator" v-if="isRefreshing">
          <Spinner :size="16" />
        </div>
      </div>

      <!-- Error -->
      <div v-if="error && !dhtStatus" class="error-state card-padded">
        <p>{{ error }}</p>
      </div>

      <!-- Loading -->
      <div v-else-if="!dhtStatus" class="loading-state">
        <Spinner :size="32" />
      </div>

      <template v-else>
        <!-- Worker Not Active Banner -->
        <div v-if="!dhtStatus.dht_running" class="inactive-banner card-padded">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" /><line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
          </svg>
          <div>
            <strong>DHT Crawler not active on this node</strong>
            <p>This worker is not included in the current node's worker configuration. Database statistics are still shown below.</p>
          </div>
        </div>

        <!-- Status Cards -->
        <div class="status-grid" :class="{ 'inactive-section': !dhtStatus.dht_running }">
          <!-- DHT Engine -->
          <div class="card-padded">
            <h3 class="card-title">DHT Engine</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Status</span>
                <span class="status-value">
                  <span class="status-dot" :class="dhtStatus.dht_running ? 'running' : 'stopped'"></span>
                  {{ dhtStatus.dht_running ? 'Running' : 'Not Active' }}
                </span>
              </div>
              <div class="status-row">
                <span class="status-label">Instances</span>
                <span class="status-value">{{ dhtStatus.instances }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Routing Table</span>
                <span class="status-value">{{ formatNumber(dhtStatus.routing_table_nodes) }} nodes ({{ formatNumber(dhtStatus.routing_table_good) }} good)</span>
              </div>
              <div class="status-row">
                <span class="status-label">Hash Queue</span>
                <span class="status-value">{{ formatNumber(dhtStatus.hash_queue_size) }}</span>
              </div>
            </div>
          </div>

          <!-- Index Stats -->
          <div class="card-padded">
            <h3 class="card-title">Index Stats</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Total Hashes</span>
                <span class="status-value">{{ formatNumber(dhtStatus.total_hashes) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Resolved</span>
                <span class="status-value seeders">{{ formatNumber(dhtStatus.resolved) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Unresolved</span>
                <span class="status-value" style="color: var(--warning)">{{ formatNumber(dhtStatus.unresolved) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Resolve Rate</span>
                <span class="status-value">{{ (dhtStatus.resolve_rate * 100).toFixed(1) }}%</span>
              </div>
            </div>
          </div>

          <!-- Sync Status -->
          <div class="card-padded">
            <h3 class="card-title">Sync Status</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Sync</span>
                <span class="status-value">
                  <span class="status-dot" :class="dhtStatus.sync_enabled ? 'running' : 'stopped'"></span>
                  {{ dhtStatus.sync_enabled ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <div class="status-row">
                <span class="status-label">Sequence</span>
                <span class="status-value">{{ formatNumber(dhtStatus.sync_sequence) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Peers</span>
                <span class="status-value">{{ dhtStatus.sync_peers }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Queue Table -->
        <div class="queue-section">
          <h3 class="section-title">Unresolved Queue</h3>
          <div class="card">
            <div class="table-wrapper">
              <table>
                <thead>
                  <tr>
                    <th>Info Hash</th>
                    <th>Source</th>
                    <th>Discovered</th>
                    <th>Attempts</th>
                    <th>Observations</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-if="queueItems.length === 0">
                    <td colspan="5" class="empty-cell">No items in queue</td>
                  </tr>
                  <tr v-for="item in queueItems" :key="item.info_hash">
                    <td>
                      <CopyText :text="item.info_hash" truncate />
                    </td>
                    <td>{{ item.source || '-' }}</td>
                    <td class="date-cell">{{ formatDate(item.discovered_at) }}</td>
                    <td>{{ item.resolve_attempts }}</td>
                    <td>{{ formatNumber(item.observations) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          <Pagination
            :current-page="queueCurrentPage"
            :total-pages="queueTotalPages"
            :total="queueTotal"
            @page="setQueuePage"
          />
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.indexer-view {
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

.status-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-bottom: 32px;
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

.queue-section {
  margin-top: 8px;
}

.table-wrapper {
  overflow-x: auto;
}

.date-cell {
  white-space: nowrap;
  color: var(--text-secondary);
  font-size: 0.8125rem;
}

.empty-cell {
  text-align: center;
  padding: 40px 12px !important;
  color: var(--text-secondary);
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

@media (max-width: 768px) {
  .status-grid {
    grid-template-columns: 1fr;
  }
}
</style>
