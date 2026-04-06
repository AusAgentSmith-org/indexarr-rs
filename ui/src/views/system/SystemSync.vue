<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { getSyncDashboard, getSyncPreferences, setSyncPreferences } from '@/api'
import type { SyncDashboard, SyncPeer } from '@/api/types'
import Spinner from '@/components/Spinner.vue'

// Dashboard state
const dashboard = ref<SyncDashboard | null>(null)
const dashLoading = ref(true)
let refreshTimer: ReturnType<typeof setInterval> | null = null

// Preferences state
const prefsLoading = ref(true)
const saving = ref(false)
const saved = ref(false)
const allCategories = ref<string[]>([])
const selectedCategories = ref<string[]>([])
const syncComments = ref(true)

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

const categoryLabels: Record<string, string> = {
  movie: 'Movies',
  tv_show: 'TV Shows',
  music: 'Music',
  ebook: 'Ebooks',
  comic: 'Comics',
  audiobook: 'Audiobooks',
  game: 'Games',
  software: 'Software',
  xxx: 'XXX',
  unknown: 'Unknown',
}

const allSelected = computed(() =>
  selectedCategories.value.length === allCategories.value.length
)

const noneSelected = computed(() =>
  selectedCategories.value.length === 0
)

function timeAgo(iso: string | null): string {
  if (!iso) return 'never'
  const diff = Date.now() - new Date(iso).getTime()
  const secs = Math.floor(diff / 1000)
  if (secs < 60) return `${secs}s ago`
  const mins = Math.floor(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ${mins % 60}m ago`
  return `${Math.floor(hrs / 24)}d ago`
}

function eventClass(event: string): string {
  switch (event) {
    case 'import': return 'event-import'
    case 'export': return 'event-export'
    case 'discovery': return 'event-discovery'
    case 'gossip': return 'event-gossip'
    case 'error': return 'event-error'
    case 'bootstrap': return 'event-bootstrap'
    default: return ''
  }
}

function healthClass(peer: { healthy: boolean; fail_count: number }): string {
  if (peer.healthy && peer.fail_count === 0) return 'health-good'
  if (peer.healthy) return 'health-warn'
  return 'health-bad'
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString()
}

// Dashboard
async function loadDashboard() {
  try {
    dashboard.value = await getSyncDashboard()
  } catch {
    // silent
  } finally {
    dashLoading.value = false
  }
}

// Preferences
function toggleCategory(cat: string) {
  const idx = selectedCategories.value.indexOf(cat)
  if (idx >= 0) {
    selectedCategories.value = selectedCategories.value.filter(c => c !== cat)
  } else {
    selectedCategories.value = [...selectedCategories.value, cat]
  }
  saved.value = false
}

function selectAll() {
  selectedCategories.value = [...allCategories.value]
  saved.value = false
}

function selectNone() {
  selectedCategories.value = []
  saved.value = false
}

async function loadPrefs() {
  prefsLoading.value = true
  try {
    const prefs = await getSyncPreferences()
    allCategories.value = prefs.all_categories
    selectedCategories.value = [...prefs.import_categories]
    syncComments.value = prefs.sync_comments ?? true
  } catch {
    // silent
  } finally {
    prefsLoading.value = false
  }
}

async function savePrefs() {
  saving.value = true
  saved.value = false
  try {
    const result = await setSyncPreferences(selectedCategories.value, syncComments.value)
    selectedCategories.value = [...result.import_categories]
    syncComments.value = result.sync_comments ?? true
    saved.value = true
  } catch {
    // silent
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  loadDashboard()
  loadPrefs()
  refreshTimer = setInterval(loadDashboard, 10000)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})
</script>

<template>
  <div class="sync-page">

    <!-- ============================================================ -->
    <!-- SYNC PREFERENCES                                              -->
    <!-- ============================================================ -->

    <div v-if="prefsLoading" class="loading-state">
      <Spinner :size="24" />
    </div>

    <template v-else>
      <div class="card-padded info-banner">
        <div class="info-icon">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
        </div>
        <div>
          <p class="info-text">
            Select which content categories to <strong>sync</strong> from the swarm when syncing with peers.
          </p>
          <p class="info-note">
            Your node will still index and contribute <strong>all categories</strong> to the swarm regardless of these settings.
            This only controls what data you pull down from other peers. Unselecting a category will remove
            previously synced data of that type from your database.
          </p>
        </div>
      </div>

      <div class="card-padded">
        <div class="section-header">
          <h3 class="section-subtitle">Sync Categories</h3>
          <div class="bulk-actions">
            <button class="btn btn-ghost btn-sm" @click="selectAll" :disabled="allSelected">Select All</button>
            <button class="btn btn-ghost btn-sm" @click="selectNone" :disabled="noneSelected">Select None</button>
          </div>
        </div>

        <div class="category-grid">
          <label
            v-for="cat in allCategories"
            :key="cat"
            class="category-item"
            :class="{ selected: selectedCategories.includes(cat) }"
          >
            <input
              type="checkbox"
              :checked="selectedCategories.includes(cat)"
              @change="toggleCategory(cat)"
              class="category-checkbox"
            />
            <div class="category-content">
              <span class="category-icon">{{ getCategoryIcon(cat) }}</span>
              <span class="category-name">{{ categoryLabels[cat] || cat }}</span>
            </div>
          </label>
        </div>

        <div class="toggle-section">
          <h3 class="section-subtitle">Social Data</h3>
          <label class="toggle-item">
            <input
              type="checkbox"
              v-model="syncComments"
              @change="saved = false"
              class="toggle-checkbox"
            />
            <div class="toggle-content">
              <span class="toggle-label">Sync comments &amp; votes</span>
              <span class="toggle-description">Import user comments and upvotes/downvotes from peers</span>
            </div>
          </label>
        </div>

        <div class="save-row">
          <div class="selected-count">
            {{ selectedCategories.length }} of {{ allCategories.length }} categories selected
          </div>
          <div class="save-actions">
            <span v-if="saved" class="save-success">Preferences saved</span>
            <button class="btn btn-accent" @click="savePrefs" :disabled="saving">
              {{ saving ? 'Saving...' : 'Save Preferences' }}
            </button>
          </div>
        </div>
      </div>

      <div v-if="noneSelected" class="card-padded warning-banner">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
        <span>No categories selected. Sync imports will be disabled until at least one category is selected.</span>
      </div>
    </template>

    <!-- ============================================================ -->
    <!-- SYNC DASHBOARD                                                -->
    <!-- ============================================================ -->

    <div v-if="dashLoading" class="loading-state">
      <Spinner :size="32" />
    </div>

    <template v-else-if="dashboard">
      <!-- Bootstrap Progress Panel -->
      <div
        v-if="dashboard.bootstrap && !bootstrapDismissed"
        class="card-padded bootstrap-panel"
      >
        <div class="bootstrap-header">
          <h3 class="section-subtitle">Initial Sync</h3>
          <button
            v-if="dashboard.bootstrap.phase === 'complete'"
            class="btn btn-ghost btn-sm"
            @click="bootstrapDismissed = true"
          >Dismiss</button>
        </div>
        <div class="bootstrap-steps">
          <div
            v-for="step in bootstrapSteps"
            :key="step.key"
            class="bootstrap-step"
            :class="'step-' + bootstrapStepStatus(step.key, dashboard.bootstrap.phase)"
          >
            <div class="step-indicator">
              <span class="step-dot"></span>
              <span v-if="step.key !== 'complete'" class="step-line"></span>
            </div>
            <div class="step-content">
              <div class="step-label">{{ step.label }}</div>
              <div class="step-detail">
                <template v-if="step.key === 'connecting'">
                  {{ dashboard.bootstrap.peers_found }} peer(s) found
                </template>
                <template v-else-if="step.key === 'downloading'">
                  {{ dashboard.bootstrap.deltas_downloaded }} / {{ dashboard.bootstrap.deltas_total || '?' }} deltas
                </template>
                <template v-else-if="step.key === 'importing'">
                  {{ dashboard.bootstrap.records_imported.toLocaleString() }}
                  <template v-if="dashboard.bootstrap.records_total > 0">
                    / {{ dashboard.bootstrap.records_total.toLocaleString() }}
                  </template>
                  records
                </template>
                <template v-else-if="step.key === 'complete'">
                  {{ step.description }}
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Overview Stats -->
      <div class="card-padded">
        <h3 class="section-subtitle">Sync Overview</h3>
        <div class="stat-grid">
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.sequence }}</div>
            <div class="stat-label">Sequence</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.total_imported.toLocaleString() }}</div>
            <div class="stat-label">Imported</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.gossip_rounds }}</div>
            <div class="stat-label">Gossip Rounds</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.peers.length }}</div>
            <div class="stat-label">Peers</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.peers.filter((p: SyncPeer) => p.healthy).length }}</div>
            <div class="stat-label">Healthy</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ dashboard.channels.length }}</div>
            <div class="stat-label">Channels</div>
          </div>
        </div>

        <div class="timing-row">
          <span class="timing-item">Last gossip: <strong>{{ timeAgo(dashboard.last_gossip_at) }}</strong></span>
          <span class="timing-item">Last export: <strong>{{ timeAgo(dashboard.last_export_at) }}</strong></span>
          <span class="timing-item">Last discovery: <strong>{{ timeAgo(dashboard.last_discovery_at) }}</strong></span>
        </div>

        <div class="config-row">
          <span class="config-item">Gossip every <strong>{{ dashboard.import_interval }}s</strong></span>
          <span class="config-item">Export every <strong>{{ dashboard.export_interval }}s</strong></span>
          <span class="config-item">Discovery every <strong>{{ dashboard.discovery_interval }}s</strong></span>
          <span class="config-item">Fanout: <strong>{{ dashboard.gossip_fanout }}</strong></span>
        </div>
      </div>

      <!-- Discovery Channels -->
      <div class="card-padded" v-if="dashboard.channels.length">
        <h3 class="section-subtitle">Discovery Channels</h3>
        <div class="channel-list">
          <div v-for="ch in dashboard.channels" :key="ch.name" class="channel-badge">
            <span class="channel-dot running"></span>
            {{ ch.name }}
          </div>
        </div>
      </div>

      <!-- Peer Table -->
      <div class="card-padded">
        <h3 class="section-subtitle">Peer Table ({{ dashboard.peers.length }}/{{ dashboard.max_peers }})</h3>
        <div v-if="dashboard.peers.length === 0" class="empty-state">
          No peers discovered yet. Waiting for discovery cycle...
        </div>
        <div v-else class="peer-table-wrap">
          <table class="peer-table">
            <thead>
              <tr>
                <th>Status</th>
                <th>Peer</th>
                <th>URL</th>
                <th>Source</th>
                <th>Sequence</th>
                <th>Fails</th>
                <th>Last Seen</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="peer in dashboard.peers" :key="peer.peer_id">
                <td>
                  <span class="health-dot" :class="healthClass(peer)"></span>
                </td>
                <td class="mono">{{ peer.peer_id }}</td>
                <td class="mono peer-url">{{ peer.url }}</td>
                <td>
                  <span class="source-badge" :class="'source-' + peer.source">{{ peer.source }}</span>
                </td>
                <td class="num">{{ peer.last_sequence }}</td>
                <td class="num" :class="{ 'fail-count': peer.fail_count > 0 }">{{ peer.fail_count }}</td>
                <td class="time-ago">{{ timeAgo(peer.last_seen) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Activity Feed -->
      <div class="card-padded">
        <h3 class="section-subtitle">Activity Feed</h3>
        <div v-if="dashboard.activity.length === 0" class="empty-state">
          No activity yet. Waiting for first sync cycle...
        </div>
        <div v-else class="activity-feed">
          <div
            v-for="(entry, idx) in dashboard.activity"
            :key="idx"
            class="activity-entry"
          >
            <span class="activity-time">{{ formatTime(entry.timestamp) }}</span>
            <span class="activity-event" :class="eventClass(entry.event)">{{ entry.event }}</span>
            <span v-if="entry.peer_id" class="activity-peer">{{ entry.peer_id }}</span>
            <span class="activity-msg">{{ entry.message }}</span>
          </div>
        </div>
      </div>
    </template>

    <div v-else class="card-padded">
      <div class="empty-state">Sync is not enabled on this node.</div>
    </div>
  </div>
</template>

<script lang="ts">
function getCategoryIcon(cat: string): string {
  const icons: Record<string, string> = {
    movie: '🎬',
    tv_show: '📺',
    music: '🎵',
    ebook: '📚',
    comic: '💬',
    audiobook: '🎧',
    game: '🎮',
    software: '💾',
    xxx: '🔞',
    unknown: '❓',
  }
  return icons[cat] || '📦'
}
</script>

<style scoped>
.sync-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

/* ── Stats Grid ─────────────────────────────────────────── */
.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}

.stat-item {
  text-align: center;
  padding: 12px 8px;
  background: var(--surface);
  border-radius: var(--radius);
  border: 1px solid var(--border);
}

.stat-value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-top: 2px;
}

/* ── Timing / Config rows ───────────────────────────────── */
.timing-row, .config-row {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.timing-row { margin-bottom: 8px; }
.config-row { padding-top: 8px; border-top: 1px solid var(--border); }

.timing-item strong, .config-item strong {
  color: var(--text-primary);
}

/* ── Channels ───────────────────────────────────────────── */
.channel-list {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.channel-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 0.8125rem;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.channel-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.channel-dot.running {
  background: var(--success);
  box-shadow: 0 0 4px var(--success);
}

/* ── Peer Table ─────────────────────────────────────────── */
.peer-table-wrap {
  overflow-x: auto;
}

.peer-table {
  width: 100%;
  font-size: 0.8125rem;
  border-collapse: collapse;
}

.peer-table th {
  text-align: left;
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  padding: 8px 12px;
  border-bottom: 2px solid var(--border);
  white-space: nowrap;
}

.peer-table td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  vertical-align: middle;
}

.peer-table tr:hover td {
  background: var(--surface);
}

.peer-table .mono {
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.peer-table .num {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.peer-table .time-ago {
  white-space: nowrap;
  color: var(--text-secondary);
}

.peer-url {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.health-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.health-good { background: var(--success); }
.health-warn { background: var(--warning); }
.health-bad { background: var(--danger, #dc3545); }

.source-badge {
  display: inline-flex;
  padding: 1px 6px;
  border-radius: 999px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
}

.source-bootstrap { background: rgba(233, 69, 96, 0.1); color: var(--accent); }
.source-pex { background: rgba(40, 167, 69, 0.1); color: var(--success); }
.source-dht { background: rgba(0, 123, 255, 0.1); color: var(--info); }
.source-xmpp { background: rgba(111, 66, 193, 0.1); color: #6f42c1; }

.fail-count { color: var(--danger, #dc3545); font-weight: 600; }

/* ── Activity Feed ──────────────────────────────────────── */
.activity-feed {
  max-height: 400px;
  overflow-y: auto;
  font-size: 0.8125rem;
  font-family: var(--font-mono);
}

.activity-entry {
  display: flex;
  gap: 8px;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
  align-items: baseline;
  flex-wrap: wrap;
}

.activity-time {
  color: var(--text-secondary);
  font-size: 0.75rem;
  flex-shrink: 0;
  min-width: 70px;
}

.activity-event {
  display: inline-flex;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  flex-shrink: 0;
}

.event-import { background: rgba(40, 167, 69, 0.15); color: var(--success); }
.event-export { background: rgba(0, 123, 255, 0.15); color: var(--info); }
.event-discovery { background: rgba(111, 66, 193, 0.15); color: #6f42c1; }
.event-gossip { background: rgba(253, 126, 20, 0.15); color: #fd7e14; }
.event-error { background: rgba(220, 53, 69, 0.15); color: var(--danger, #dc3545); }
.event-bootstrap { background: rgba(23, 162, 184, 0.15); color: #17a2b8; }

.activity-peer {
  color: var(--accent);
  font-size: 0.75rem;
  flex-shrink: 0;
}

.activity-msg {
  color: var(--text-primary);
  word-break: break-word;
}

.empty-state {
  color: var(--text-secondary);
  font-size: 0.875rem;
  padding: 24px 0;
  text-align: center;
}

/* ── Preferences (carried over) ─────────────────────────── */
.info-banner {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}

.info-icon {
  flex-shrink: 0;
  color: var(--info);
  margin-top: 2px;
}

.info-text {
  font-size: 0.9375rem;
  line-height: 1.5;
}

.info-note {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  margin-top: 4px;
  line-height: 1.5;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.bulk-actions {
  display: flex;
  gap: 4px;
}

.btn-sm {
  padding: 4px 10px;
  font-size: 0.75rem;
}

.category-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 8px;
}

.category-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}

.category-item:hover { border-color: var(--text-secondary); }
.category-item.selected { border-color: var(--accent); background: rgba(233, 69, 96, 0.05); }
.category-checkbox { display: none; }

.category-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon { font-size: 1.25rem; line-height: 1; }
.category-name { font-size: 0.875rem; font-weight: 500; }

.toggle-section {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.toggle-section .section-subtitle { margin-bottom: 12px; }

.toggle-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  cursor: pointer;
  user-select: none;
}

.toggle-checkbox {
  margin-top: 2px;
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  flex-shrink: 0;
}

.toggle-content { display: flex; flex-direction: column; gap: 2px; }
.toggle-label { font-size: 0.875rem; font-weight: 500; }
.toggle-description { font-size: 0.8125rem; color: var(--text-secondary); }

.save-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.selected-count { font-size: 0.8125rem; color: var(--text-secondary); }
.save-actions { display: flex; align-items: center; gap: 12px; }
.save-success { font-size: 0.8125rem; color: var(--success); font-weight: 500; }

.warning-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--warning);
  font-size: 0.875rem;
}

.warning-banner svg { flex-shrink: 0; }

/* ── Bootstrap Panel ───────────────────────────────────── */
.bootstrap-panel {
  border: 1px solid rgba(23, 162, 184, 0.3);
  background: rgba(23, 162, 184, 0.03);
}

.bootstrap-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
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

/* Step states */
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
  color: var(--text-primary);
}

.step-active .step-detail {
  color: var(--text-primary);
}

@keyframes pulse-dot {
  0%, 100% { box-shadow: 0 0 0 0 rgba(23, 162, 184, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(23, 162, 184, 0); }
}

@media (max-width: 640px) {
  .stat-grid { grid-template-columns: repeat(3, 1fr); }
  .category-grid { grid-template-columns: 1fr 1fr; }
  .save-row { flex-direction: column; gap: 12px; align-items: flex-start; }
  .timing-row, .config-row { flex-direction: column; gap: 4px; }
}
</style>
