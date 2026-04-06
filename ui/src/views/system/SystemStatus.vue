<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getSystemStatus, getIdentityStatus, getEpochInfo } from '@/api'
import type { SystemStatus, IdentityStatus, EpochInfo } from '@/api'
import Spinner from '@/components/Spinner.vue'

const status = ref<SystemStatus | null>(null)
const identity = ref<IdentityStatus | null>(null)
const epoch = ref<EpochInfo | null>(null)
const loading = ref(true)

const uptime = computed(() => {
  if (!status.value) return '-'
  const secs = Math.floor(status.value.uptime_seconds)
  const days = Math.floor(secs / 86400)
  const hrs = Math.floor((secs % 86400) / 3600)
  const mins = Math.floor((secs % 3600) / 60)
  const parts: string[] = []
  if (days > 0) parts.push(`${days}d`)
  if (hrs > 0) parts.push(`${hrs}h`)
  parts.push(`${mins}m`)
  return parts.join(' ')
})

async function load() {
  loading.value = true
  try {
    const [s, i, e] = await Promise.all([
      getSystemStatus(),
      getIdentityStatus(),
      getEpochInfo(),
    ])
    status.value = s
    identity.value = i
    epoch.value = e
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="status-page">
    <div v-if="loading" class="loading-state">
      <Spinner :size="32" />
    </div>
    <template v-else-if="status">
      <div class="card">
        <table class="info-table">
          <tbody>
            <tr>
              <td class="info-label">Version</td>
              <td class="info-value">{{ status.version }}</td>
            </tr>
            <tr>
              <td class="info-label">Python</td>
              <td class="info-value">{{ status.python_version }}</td>
            </tr>
            <tr>
              <td class="info-label">Docker</td>
              <td class="info-value">
                <span :class="status.is_docker ? 'badge-yes' : 'badge-no'">
                  {{ status.is_docker ? 'Yes' : 'No' }}
                </span>
              </td>
            </tr>
            <tr>
              <td class="info-label">Database</td>
              <td class="info-value">{{ status.db_backend }} — {{ status.db_version }}</td>
            </tr>
            <tr>
              <td class="info-label">Data Directory</td>
              <td class="info-value"><code>{{ status.data_dir }}</code></td>
            </tr>
            <tr>
              <td class="info-label">Host</td>
              <td class="info-value">{{ status.host }}:{{ status.port }}</td>
            </tr>
            <tr>
              <td class="info-label">Workers</td>
              <td class="info-value">
                <span v-for="w in status.workers" :key="w" class="worker-badge">{{ w }}</span>
              </td>
            </tr>
            <tr>
              <td class="info-label">Debug Mode</td>
              <td class="info-value">
                <span :class="status.debug ? 'badge-yes' : 'badge-no'">
                  {{ status.debug ? 'Enabled' : 'Disabled' }}
                </span>
              </td>
            </tr>
            <tr>
              <td class="info-label">Total Hashes</td>
              <td class="info-value">{{ status.total_hashes.toLocaleString() }}</td>
            </tr>
            <tr>
              <td class="info-label">Resolved</td>
              <td class="info-value">{{ status.resolved_hashes.toLocaleString() }}</td>
            </tr>
            <tr>
              <td class="info-label">Uptime</td>
              <td class="info-value">{{ uptime }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Identity -->
      <div class="card" v-if="identity">
        <table class="info-table">
          <tbody>
            <tr>
              <td class="info-label">Contributor ID</td>
              <td class="info-value"><code>{{ identity.contributor_id }}</code></td>
            </tr>
            <tr>
              <td class="info-label">Public Key</td>
              <td class="info-value">
                <code class="truncated-key">{{ identity.public_key }}</code>
              </td>
            </tr>
            <tr v-if="identity.recovery_key">
              <td class="info-label">Recovery Key</td>
              <td class="info-value">
                <div class="recovery-key-box">
                  <code>{{ identity.recovery_key }}</code>
                  <p class="recovery-warning">Save this key! It won't be shown again.</p>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Epoch -->
      <div class="card" v-if="epoch">
        <table class="info-table">
          <tbody>
            <tr>
              <td class="info-label">Swarm Epoch</td>
              <td class="info-value"><strong>{{ epoch.epoch }}</strong></td>
            </tr>
            <tr>
              <td class="info-label">Epoch Reason</td>
              <td class="info-value">{{ epoch.reason }}</td>
            </tr>
            <tr>
              <td class="info-label">Effective Since</td>
              <td class="info-value">{{ epoch.effective_at }}</td>
            </tr>
            <tr>
              <td class="info-label">Grace Period</td>
              <td class="info-value">{{ epoch.grace_hours }} hours</td>
            </tr>
            <tr>
              <td class="info-label">Purge Policy</td>
              <td class="info-value">{{ epoch.purge_policy }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="card-padded about-section">
        <h3 class="section-subtitle">About</h3>
        <p>Indexarr is a decentralized torrent indexing system with DHT crawling, content classification, and P2P sync.</p>
        <p style="margin-top: 8px; color: var(--text-secondary); font-size: 0.875rem;">
          Built with FastAPI, SQLAlchemy, Vue 3, and btdht.
        </p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.status-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 60px 0;
}

.info-table {
  width: 100%;
}

.info-table tr:hover td {
  background: var(--surface);
}

.info-label {
  width: 180px;
  font-weight: 600;
  color: var(--text-secondary);
  font-size: 0.875rem;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
}

.info-value {
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
  font-size: 0.875rem;
}

.info-value code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  padding: 2px 6px;
  background: var(--surface);
  border-radius: var(--radius-sm);
}

.badge-yes {
  display: inline-flex;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  background: rgba(40, 167, 69, 0.1);
  color: var(--success);
}

.badge-no {
  display: inline-flex;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  background: var(--surface);
  color: var(--text-secondary);
}

.worker-badge {
  display: inline-flex;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-weight: 500;
  background: var(--surface);
  border: 1px solid var(--border);
  margin-right: 4px;
  margin-bottom: 2px;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.about-section p {
  font-size: 0.9375rem;
  line-height: 1.6;
}

.truncated-key {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  word-break: break-all;
  display: block;
  max-width: 400px;
}

.recovery-key-box {
  background: var(--surface);
  border: 2px solid var(--warning);
  border-radius: var(--radius);
  padding: 12px;
}

.recovery-key-box code {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  word-break: break-all;
  display: block;
  margin-bottom: 8px;
}

.recovery-warning {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--warning);
}
</style>
