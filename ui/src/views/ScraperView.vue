<script setup lang="ts">
import { ref, computed } from 'vue'
import { getScraperStatus } from '@/api'
import type { ScraperStatus } from '@/api'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { useFormatters } from '@/composables/useFormatters'
import Spinner from '@/components/Spinner.vue'

const { formatNumber } = useFormatters()

const status = ref<ScraperStatus | null>(null)
const error = ref<string | null>(null)

async function loadStatus() {
  try {
    status.value = await getScraperStatus()
    error.value = null
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load scraper status'
  }
}

const { isRefreshing } = useAutoRefresh(loadStatus, 10000)

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${Math.floor(seconds)}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h}h ${m}m`
}

function formatAgo(seconds: number | null): string {
  if (seconds === null) return 'Never'
  if (seconds < 60) return `${Math.floor(seconds)}s ago`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
  return `${Math.floor(seconds / 3600)}h ago`
}

const sourceColors: Record<string, string> = {
  'api:tpb': '#e74c3c',
  'api:eztv': '#3498db',
  'api:nyaa': '#9b59b6',
  'api:knaben': '#2ecc71',
  'api:yts': '#e67e22',
  'api:torrentscsv': '#1abc9c',
  'api:animetosho': '#f39c12',
  'api:subsplease': '#e91e63',
  'api:archive': '#795548',
  'api:therarbg': '#607d8b',
  'api:limetorrents': '#8bc34a',
  'api:sukebei': '#ff5722',
  'api:fitgirl': '#00bcd4',
  'api:1337x': '#ff9800',
}

const totalFromSources = computed(() => {
  if (!status.value) return 0
  return status.value.source_counts.reduce((sum, s) => sum + s.count, 0)
})
</script>

<template>
  <div class="scraper-view">
    <div class="container">
      <div class="page-header">
        <h1 class="page-title">Scraper</h1>
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
            <strong>Scraper not active on this node</strong>
            <p>This worker is not included in the current node's worker configuration. Database statistics are still shown below.</p>
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
              <div class="status-row">
                <span class="status-label">Interval</span>
                <span class="status-value">{{ status.interval }}s</span>
              </div>
            </div>
          </div>

          <div class="card-padded">
            <h3 class="card-title">Cycle Stats</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Current Cycle</span>
                <span class="status-value">{{ formatNumber(status.cycle) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Last Cycle</span>
                <span class="status-value">{{ formatAgo(status.last_cycle_ago) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Last Cycle Hashes</span>
                <span class="status-value">{{ formatNumber(status.last_cycle_total) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Total Scraped (Session)</span>
                <span class="status-value seeders">{{ formatNumber(status.total_scraped) }}</span>
              </div>
            </div>
          </div>

          <div class="card-padded">
            <h3 class="card-title">Database Totals</h3>
            <div class="status-items">
              <div class="status-row">
                <span class="status-label">Total from Scrapers</span>
                <span class="status-value">{{ formatNumber(totalFromSources) }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">Active Sources</span>
                <span class="status-value">{{ status.enabled_sources.length }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Last Cycle Results -->
        <div class="section" v-if="status.last_cycle_results.length > 0">
          <h3 class="section-title">Last Cycle Results</h3>
          <div class="source-grid">
            <div
              v-for="src in status.last_cycle_results"
              :key="src.name"
              class="source-card card-padded"
              :class="{ 'source-error': src.last_count === -1 }"
            >
              <div class="source-name">{{ src.name }}</div>
              <div class="source-count" :class="{ error: src.last_count === -1 }">
                {{ src.last_count === -1 ? 'Error' : formatNumber(src.last_count) }}
              </div>
            </div>
          </div>
        </div>

        <!-- Database Source Breakdown -->
        <div class="section" v-if="status.source_counts.length > 0">
          <h3 class="section-title">Source Breakdown (All Time)</h3>
          <div class="card">
            <div class="table-wrapper">
              <table>
                <thead>
                  <tr>
                    <th>Source</th>
                    <th>Hashes</th>
                    <th>Share</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="src in status.source_counts" :key="src.source">
                    <td>
                      <span class="source-badge" :style="{ background: sourceColors[src.source] || '#6c757d' }"></span>
                      {{ src.source }}
                    </td>
                    <td>{{ formatNumber(src.count) }}</td>
                    <td>{{ totalFromSources > 0 ? ((src.count / totalFromSources) * 100).toFixed(1) + '%' : '-' }}</td>
                    <td class="bar-cell">
                      <div class="bar-track">
                        <div
                          class="bar-fill"
                          :style="{
                            width: totalFromSources > 0 ? (src.count / totalFromSources * 100) + '%' : '0%',
                            background: sourceColors[src.source] || '#6c757d',
                          }"
                        ></div>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Enabled Sources List -->
        <div class="section">
          <h3 class="section-title">Enabled Sources</h3>
          <div class="enabled-sources">
            <span
              v-for="src in status.enabled_sources"
              :key="src"
              class="source-tag"
            >{{ src }}</span>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.scraper-view {
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

.section {
  margin-bottom: 32px;
}

.section-title {
  font-size: 1rem;
  font-weight: 600;
  margin-bottom: 12px;
}

/* Source grid for last cycle results */
.source-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}

.source-card {
  text-align: center;
  padding: 16px 12px !important;
}

.source-card.source-error {
  border-left: 3px solid var(--accent);
}

.source-name {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.source-count {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text);
}

.source-count.error {
  color: var(--accent);
  font-size: 0.875rem;
}

/* Table styles */
.table-wrapper {
  overflow-x: auto;
}

.source-badge {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 2px;
  margin-right: 8px;
  vertical-align: middle;
}

.bar-cell {
  width: 30%;
  min-width: 120px;
}

.bar-track {
  height: 6px;
  background: var(--surface);
  border-radius: 3px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

/* Enabled sources */
.enabled-sources {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.source-tag {
  padding: 4px 12px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 0.8125rem;
  font-weight: 500;
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
  .source-grid {
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  }
}
</style>
