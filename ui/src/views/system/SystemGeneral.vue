<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getSystemStatus, restoreIdentity, getIdentityStatus } from '@/api'
import type { SystemStatus, IdentityStatus } from '@/api'
import Spinner from '@/components/Spinner.vue'

const status = ref<SystemStatus | null>(null)
const identity = ref<IdentityStatus | null>(null)
const recoveryKey = ref('')
const restoring = ref(false)
const restoreError = ref<string | null>(null)
const restoreSuccess = ref(false)
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    status.value = await getSystemStatus()
    identity.value = await getIdentityStatus()
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

async function handleRestore() {
  if (!recoveryKey.value.trim()) return
  restoring.value = true
  restoreError.value = null
  restoreSuccess.value = false
  try {
    identity.value = await restoreIdentity(recoveryKey.value)
    restoreSuccess.value = true
    recoveryKey.value = ''
  } catch (err: any) {
    restoreError.value = err?.message || 'Failed to restore identity'
  } finally {
    restoring.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="general-page">
    <div v-if="loading" class="loading-state">
      <Spinner :size="32" />
    </div>
    <template v-else-if="status">
      <div class="card-padded">
        <h3 class="section-subtitle">Server Configuration</h3>
        <p class="config-note">Configuration is managed via environment variables with the <code>INDEXARR_</code> prefix. See the documentation for all available options.</p>
        <div class="config-grid">
          <div class="config-item">
            <span class="config-label">Bind Address</span>
            <span class="config-value">{{ status.host }}</span>
          </div>
          <div class="config-item">
            <span class="config-label">Port</span>
            <span class="config-value">{{ status.port }}</span>
          </div>
          <div class="config-item">
            <span class="config-label">Database Backend</span>
            <span class="config-value">{{ status.db_backend }}</span>
          </div>
          <div class="config-item">
            <span class="config-label">Data Directory</span>
            <span class="config-value"><code>{{ status.data_dir }}</code></span>
          </div>
          <div class="config-item">
            <span class="config-label">Debug Mode</span>
            <span class="config-value">{{ status.debug ? 'Enabled' : 'Disabled' }}</span>
          </div>
          <div class="config-item">
            <span class="config-label">Active Workers</span>
            <span class="config-value">{{ status.workers.join(', ') }}</span>
          </div>
        </div>
      </div>

      <div class="card-padded env-section">
        <h3 class="section-subtitle">Environment Variables</h3>
        <p class="config-note">Key environment variables for configuration:</p>
        <table class="env-table">
          <thead>
            <tr>
              <th>Variable</th>
              <th>Description</th>
            </tr>
          </thead>
          <tbody>
            <tr><td><code>INDEXARR_DB_BACKEND</code></td><td>Database backend: postgresql or sqlite</td></tr>
            <tr><td><code>INDEXARR_DB_URL</code></td><td>Database connection URL</td></tr>
            <tr><td><code>INDEXARR_HOST</code></td><td>HTTP server bind address</td></tr>
            <tr><td><code>INDEXARR_PORT</code></td><td>HTTP server port</td></tr>
            <tr><td><code>INDEXARR_WORKERS</code></td><td>Comma-separated list of workers to run</td></tr>
            <tr><td><code>INDEXARR_DHT_INSTANCES</code></td><td>Number of DHT node instances</td></tr>
            <tr><td><code>INDEXARR_RESOLVE_WORKERS</code></td><td>Number of resolver threads</td></tr>
            <tr><td><code>INDEXARR_TORZNAB_API_KEY</code></td><td>API key for Torznab authentication</td></tr>
            <tr><td><code>INDEXARR_SYNC_ENABLED</code></td><td>Enable P2P sync worker</td></tr>
            <tr><td><code>INDEXARR_DEBUG</code></td><td>Enable debug mode</td></tr>
          </tbody>
        </table>
      </div>

      <div class="card-padded" v-if="identity">
        <h3 class="section-subtitle">Contributor Identity</h3>
        <div class="config-grid">
          <div class="config-item">
            <span class="config-label">Contributor ID</span>
            <span class="config-value"><code>{{ identity.contributor_id }}</code></span>
          </div>
          <div class="config-item">
            <span class="config-label">Status</span>
            <span class="config-value">{{ identity.initialized ? 'Active' : 'Not initialized' }}</span>
          </div>
        </div>

        <div class="restore-section">
          <h4 class="restore-title">Restore Identity</h4>
          <p class="config-note">Enter your recovery key to restore a previous contributor identity.</p>
          <div class="restore-form">
            <input
              v-model="recoveryKey"
              type="text"
              placeholder="XXXX-XXXX-XXXX-XXXX-..."
              class="restore-input"
            />
            <button class="btn btn-accent" @click="handleRestore" :disabled="restoring || !recoveryKey.trim()">
              {{ restoring ? 'Restoring...' : 'Restore' }}
            </button>
          </div>
          <p v-if="restoreError" class="restore-error">{{ restoreError }}</p>
          <p v-if="restoreSuccess" class="restore-success">Identity restored successfully!</p>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.general-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 60px 0;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.config-note {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-bottom: 16px;
}

.config-note code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  padding: 1px 4px;
  background: var(--surface);
  border-radius: var(--radius-sm);
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}

.config-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.config-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.config-value {
  font-size: 0.9375rem;
  font-weight: 500;
}

.config-value code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.env-section {
  margin-top: 0;
}

.env-table {
  width: 100%;
  border-collapse: collapse;
}

.env-table th {
  text-align: left;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  padding: 8px 12px;
  border-bottom: 2px solid var(--border);
}

.env-table td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  font-size: 0.875rem;
}

.env-table td code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  padding: 1px 4px;
  background: var(--surface);
  border-radius: var(--radius-sm);
  white-space: nowrap;
}

.restore-section {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.restore-title {
  font-size: 0.8125rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.restore-form {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.restore-input {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  outline: none;
  flex: 1;
  max-width: 400px;
}

.restore-input:focus {
  border-color: var(--accent);
}

.restore-error {
  font-size: 0.8125rem;
  color: var(--accent);
  margin-top: 8px;
}

.restore-success {
  font-size: 0.8125rem;
  color: var(--success);
  margin-top: 8px;
}
</style>
