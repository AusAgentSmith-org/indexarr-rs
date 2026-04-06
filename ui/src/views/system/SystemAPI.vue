<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getApiKey, generateApiKey } from '@/api'
import type { ApiKeyResponse } from '@/api'
import CopyText from '@/components/CopyText.vue'
import Spinner from '@/components/Spinner.vue'

const apiKeyData = ref<ApiKeyResponse | null>(null)
const generatedKey = ref<string | null>(null)
const loading = ref(true)
const generating = ref(false)

const displayKey = computed(() => {
  if (generatedKey.value) return generatedKey.value
  if (apiKeyData.value?.has_key) return apiKeyData.value.key
  return null
})

async function load() {
  loading.value = true
  try {
    apiKeyData.value = await getApiKey()
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

async function handleGenerate() {
  if (!confirm('Generate a new API key? The old key will stop working.')) return
  generating.value = true
  try {
    const result = await generateApiKey()
    generatedKey.value = result.key
    apiKeyData.value = { key: result.key, has_key: true }
  } catch {
    // silent
  } finally {
    generating.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="api-page">
    <!-- Prowlarr Integration Guide -->
    <div class="card-padded">
      <h3 class="section-subtitle">Integrating with Prowlarr</h3>
      <p class="guide-intro">
        Indexarr provides a Torznab-compatible API that integrates directly with
        <a href="https://prowlarr.com/" target="_blank" rel="noopener">Prowlarr</a>,
        Sonarr, Radarr, and other *arr applications.
      </p>

      <ol class="setup-steps">
        <li>
          <strong>Open Prowlarr</strong> and navigate to
          <em>Settings &rarr; Indexers &rarr; Add Indexer</em>
        </li>
        <li>
          Select <strong>"Generic Newznab"</strong> or <strong>"Torznab"</strong> as the indexer type
        </li>
        <li>
          Set the <strong>URL</strong> to your Indexarr instance:
          <code class="url-example">http://&lt;your-host&gt;:8080/api/torznab</code>
        </li>
        <li>
          Enter your <strong>API Key</strong> from the section below
          <span class="step-note">(generate one if you haven't already)</span>
        </li>
        <li>
          Click <strong>Test</strong> to verify the connection, then <strong>Save</strong>
        </li>
      </ol>

      <div class="prowlarr-note">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        <span>
          The API key is configured on the <router-link to="/system/general">General</router-link> settings page
          via the <code>INDEXARR_TORZNAB_API_KEY</code> environment variable, or generated below.
        </span>
      </div>
    </div>

    <!-- API Key Management -->
    <div class="card-padded">
      <h3 class="section-subtitle">API Key</h3>

      <div v-if="loading" class="loading-inline">
        <Spinner :size="20" />
      </div>

      <template v-else>
        <div v-if="displayKey" class="key-display">
          <div class="key-row">
            <CopyText :text="displayKey" />
            <button class="btn btn-ghost" @click="handleGenerate" :disabled="generating">
              {{ generating ? 'Generating...' : 'Regenerate' }}
            </button>
          </div>
          <p v-if="generatedKey" class="key-warning">
            Save this key now — it won't be shown in full again.
          </p>
        </div>
        <div v-else class="no-key">
          <p>No API key configured. Generate one to enable authenticated access.</p>
          <button class="btn btn-accent" @click="handleGenerate" :disabled="generating" style="margin-top: 12px;">
            {{ generating ? 'Generating...' : 'Generate API Key' }}
          </button>
        </div>
      </template>
    </div>

    <!-- API Documentation -->
    <div class="card-padded">
      <h3 class="section-subtitle">API Documentation</h3>
      <p class="api-desc">
        Indexarr exposes a full REST API with interactive documentation powered by Swagger/OpenAPI.
      </p>
      <div class="api-links">
        <a href="/docs" target="_blank" class="btn btn-accent">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" />
          </svg>
          Open Swagger UI
        </a>
        <a href="/redoc" target="_blank" class="btn btn-ghost">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" />
          </svg>
          Open ReDoc
        </a>
        <a href="/openapi.json" target="_blank" class="btn btn-ghost">
          OpenAPI JSON
        </a>
      </div>

      <div class="endpoints-summary">
        <h4>Available Endpoints</h4>
        <table class="endpoint-table">
          <thead>
            <tr>
              <th>Method</th>
              <th>Path</th>
              <th>Description</th>
            </tr>
          </thead>
          <tbody>
            <tr><td><span class="method get">GET</span></td><td>/api/v1/search</td><td>Full-text search with facets</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/api/v1/recent</td><td>Recently indexed torrents</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/api/v1/stats</td><td>Index statistics</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/api/v1/torrent/{hash}</td><td>Torrent detail</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/api/v1/queue</td><td>Unresolved queue</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/api/torznab</td><td>Torznab API (Prowlarr/Sonarr)</td></tr>
            <tr><td><span class="method post">POST</span></td><td>/api/v1/import</td><td>Bulk import</td></tr>
            <tr><td><span class="method get">GET</span></td><td>/graphql</td><td>GraphQL playground</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style scoped>
.api-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.guide-intro {
  font-size: 0.9375rem;
  line-height: 1.6;
  margin-bottom: 16px;
}

.setup-steps {
  list-style: none;
  padding: 0;
  counter-reset: step;
}

.setup-steps li {
  counter-increment: step;
  position: relative;
  padding: 12px 12px 12px 48px;
  border-left: 2px solid var(--border);
  margin-left: 14px;
  font-size: 0.9375rem;
  line-height: 1.6;
}

.setup-steps li::before {
  content: counter(step);
  position: absolute;
  left: -15px;
  top: 10px;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  font-size: 0.8125rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}

.setup-steps li:last-child {
  border-left-color: transparent;
}

.url-example {
  display: inline-block;
  margin-top: 4px;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  padding: 4px 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}

.step-note {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.prowlarr-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 16px;
  padding: 12px 16px;
  background: var(--surface);
  border-radius: var(--radius);
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.prowlarr-note svg {
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--info);
}

.prowlarr-note code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  padding: 1px 4px;
  background: var(--card);
  border-radius: var(--radius-sm);
}

.loading-inline {
  display: flex;
  padding: 20px 0;
}

.key-display {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.key-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.key-warning {
  font-size: 0.8125rem;
  color: var(--warning);
  font-weight: 500;
}

.no-key p {
  font-size: 0.9375rem;
  color: var(--text-secondary);
}

.api-desc {
  font-size: 0.9375rem;
  margin-bottom: 16px;
  line-height: 1.6;
}

.api-links {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 24px;
}

.endpoints-summary h4 {
  font-size: 0.8125rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.endpoint-table {
  width: 100%;
}

.endpoint-table th {
  font-size: 0.75rem;
  padding: 8px 12px;
}

.endpoint-table td {
  font-size: 0.875rem;
  padding: 6px 12px;
  font-family: var(--font-mono);
}

.endpoint-table td:last-child {
  font-family: var(--font-sans);
  color: var(--text-secondary);
}

.method {
  display: inline-flex;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  font-size: 0.6875rem;
  font-weight: 700;
  font-family: var(--font-mono);
}

.method.get {
  background: rgba(40, 167, 69, 0.1);
  color: var(--success);
}

.method.post {
  background: rgba(23, 162, 184, 0.1);
  color: var(--info);
}
</style>
