<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { uploadTorrents } from '@/api'
import type { UploadResultItem } from '@/api'
import { useFormatters } from '@/composables/useFormatters'
import Spinner from '@/components/Spinner.vue'
import Badge from '@/components/Badge.vue'

const router = useRouter()
const { formatBytes } = useFormatters()

const selectedFiles = ref<File[]>([])
const isDragging = ref(false)
const uploading = ref(false)
const results = ref<UploadResultItem[] | null>(null)
const summary = ref<{ imported: number; duplicates: number; errors: number } | null>(null)
const error = ref<string | null>(null)
const warningAccepted = ref(false)

const hasFiles = computed(() => selectedFiles.value.length > 0)

function onDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function onDragLeave() {
  isDragging.value = false
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  if (e.dataTransfer?.files) {
    addFiles(Array.from(e.dataTransfer.files))
  }
}

function onFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  if (input.files) {
    addFiles(Array.from(input.files))
  }
  input.value = ''
}

function addFiles(files: File[]) {
  const torrentFiles = files.filter(f => f.name.toLowerCase().endsWith('.torrent'))
  if (torrentFiles.length === 0) return
  // Deduplicate by name
  const existing = new Set(selectedFiles.value.map(f => f.name))
  for (const f of torrentFiles) {
    if (!existing.has(f.name)) {
      selectedFiles.value.push(f)
      existing.add(f.name)
    }
  }
}

function removeFile(index: number) {
  selectedFiles.value.splice(index, 1)
}

function clearAll() {
  selectedFiles.value = []
  results.value = null
  summary.value = null
  error.value = null
  warningAccepted.value = false
}

async function doUpload() {
  if (!hasFiles.value || !warningAccepted.value) return
  uploading.value = true
  error.value = null
  results.value = null
  summary.value = null

  try {
    const resp = await uploadTorrents(selectedFiles.value)
    results.value = resp.results
    summary.value = { imported: resp.imported, duplicates: resp.duplicates, errors: resp.errors }
    selectedFiles.value = []
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Upload failed'
  } finally {
    uploading.value = false
  }
}

function viewTorrent(hash: string) {
  if (hash) router.push(`/torrent/${hash}`)
}
</script>

<template>
  <div class="upload-view">
    <div class="container">
      <h1 class="page-title">Upload Torrents</h1>
      <p class="page-desc">
        Upload <code>.torrent</code> files to add them to your index. Only metadata is extracted and stored &mdash;
        the <code>.torrent</code> file itself is not kept or transferred.
      </p>

      <!-- Warnings -->
      <div class="warning-card">
        <div class="warning-icon">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
            <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
          </svg>
        </div>
        <div class="warning-body">
          <strong>Before you upload</strong>
          <ul>
            <li>All uploaded torrent metadata will be <strong>synced to the swarm</strong> and visible to all peers.</li>
            <li>Uploading <strong>private tracker</strong> torrents may result in your account being banned by those trackers. Private torrents are flagged but still indexed.</li>
          </ul>
          <label class="warning-accept">
            <input type="checkbox" v-model="warningAccepted" />
            I understand and accept these terms
          </label>
        </div>
      </div>

      <!-- Drop zone (only when no results) -->
      <template v-if="!results">
        <div
          class="drop-zone"
          :class="{ 'drop-zone-active': isDragging, 'drop-zone-has-files': hasFiles }"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
          @click="($refs.fileInput as HTMLInputElement)?.click()"
        >
          <input
            ref="fileInput"
            type="file"
            accept=".torrent"
            multiple
            class="file-input"
            @change="onFileSelect"
          />
          <div class="drop-icon">
            <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
          </div>
          <p class="drop-text">
            <strong>Drop .torrent files here</strong> or click to browse
          </p>
          <p class="drop-hint">Supports multiple files. Only .torrent files accepted.</p>
        </div>

        <!-- Selected files list -->
        <div v-if="hasFiles" class="file-list card">
          <div class="file-list-header">
            <h3>Selected Files ({{ selectedFiles.length }})</h3>
            <button class="btn btn-ghost btn-sm" @click="clearAll">Clear All</button>
          </div>
          <div class="file-list-items">
            <div v-for="(file, idx) in selectedFiles" :key="file.name" class="file-item">
              <span class="file-name">{{ file.name }}</span>
              <span class="file-size">{{ formatBytes(file.size) }}</span>
              <button class="btn-remove" @click.stop="removeFile(idx)" title="Remove">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Upload button -->
          <div class="upload-actions">
            <button
              class="btn btn-accent btn-lg"
              :disabled="!warningAccepted || uploading"
              @click="doUpload"
            >
              <Spinner v-if="uploading" :size="16" />
              <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" />
              </svg>
              {{ uploading ? 'Uploading...' : `Upload ${selectedFiles.length} file${selectedFiles.length !== 1 ? 's' : ''}` }}
            </button>
            <span v-if="!warningAccepted" class="upload-hint">Accept the warning above to upload</span>
          </div>
        </div>

        <!-- Error -->
        <div v-if="error" class="error-card card-padded">
          <p>{{ error }}</p>
        </div>
      </template>

      <!-- Results -->
      <template v-if="results">
        <div class="results-summary card-padded">
          <h2>Upload Complete</h2>
          <div class="summary-stats">
            <div class="summary-stat imported" v-if="summary?.imported">
              <span class="stat-num">{{ summary.imported }}</span>
              <span class="stat-label">Imported</span>
            </div>
            <div class="summary-stat duplicate" v-if="summary?.duplicates">
              <span class="stat-num">{{ summary.duplicates }}</span>
              <span class="stat-label">Duplicates</span>
            </div>
            <div class="summary-stat errored" v-if="summary?.errors">
              <span class="stat-num">{{ summary.errors }}</span>
              <span class="stat-label">Errors</span>
            </div>
          </div>
        </div>

        <div class="results-table card">
          <table>
            <thead>
              <tr>
                <th>Status</th>
                <th>Name</th>
                <th>Size</th>
                <th>Category</th>
                <th>Private</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in results" :key="r.info_hash || r.name" :class="`row-${r.status}`">
                <td>
                  <span class="status-badge" :class="`status-${r.status}`">{{ r.status }}</span>
                </td>
                <td class="name-cell">
                  <span>{{ r.name }}</span>
                  <span v-if="r.message" class="result-message">{{ r.message }}</span>
                </td>
                <td>{{ r.size ? formatBytes(r.size) : '-' }}</td>
                <td>
                  <Badge v-if="r.content_type" :type="r.content_type" />
                  <span v-else>-</span>
                </td>
                <td>
                  <span v-if="r.private" class="private-flag">Yes</span>
                  <span v-else>-</span>
                </td>
                <td>
                  <button
                    v-if="r.status === 'imported' && r.info_hash"
                    class="btn btn-ghost btn-sm"
                    @click="viewTorrent(r.info_hash)"
                  >
                    View
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="results-actions">
          <button class="btn btn-accent" @click="clearAll">Upload More</button>
          <router-link to="/" class="btn btn-ghost">Back to Search</router-link>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.upload-view {
  padding-bottom: 40px;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 700;
  margin-top: 24px;
}

.page-desc {
  color: var(--text-secondary);
  font-size: 0.9375rem;
  margin-top: 8px;
  margin-bottom: 20px;
}

.page-desc code {
  background: var(--surface);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.8125rem;
}

/* Warning card */
.warning-card {
  display: flex;
  gap: 16px;
  padding: 16px 20px;
  background: color-mix(in srgb, var(--warning, #f59e0b) 10%, var(--card));
  border: 1px solid color-mix(in srgb, var(--warning, #f59e0b) 30%, var(--border));
  border-radius: var(--radius-lg, 12px);
  margin-bottom: 20px;
}

.warning-icon {
  color: var(--warning, #f59e0b);
  flex-shrink: 0;
  margin-top: 2px;
}

.warning-body {
  font-size: 0.875rem;
  line-height: 1.6;
}

.warning-body strong {
  display: block;
  margin-bottom: 8px;
  font-size: 0.9375rem;
}

.warning-body ul {
  margin: 0;
  padding-left: 20px;
}

.warning-body li {
  margin-bottom: 4px;
  color: var(--text-secondary);
}

.warning-body li strong {
  display: inline;
  font-size: inherit;
  margin: 0;
  color: var(--text);
}

.warning-accept {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  cursor: pointer;
  font-weight: 500;
  color: var(--text);
}

.warning-accept input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
}

/* Drop zone */
.drop-zone {
  border: 2px dashed var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 48px 24px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease;
  background: var(--card);
}

.drop-zone:hover,
.drop-zone-active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 5%, var(--card));
}

.drop-zone-active {
  border-style: solid;
}

.file-input {
  display: none;
}

.drop-icon {
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.drop-zone:hover .drop-icon,
.drop-zone-active .drop-icon {
  color: var(--accent);
}

.drop-text {
  font-size: 1rem;
  margin-bottom: 4px;
}

.drop-hint {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

/* File list */
.file-list {
  margin-top: 16px;
}

.file-list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
}

.file-list-header h3 {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.file-list-items {
  max-height: 320px;
  overflow-y: auto;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border);
}

.file-item:last-child {
  border-bottom: none;
}

.file-name {
  flex: 1;
  font-size: 0.875rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.btn-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius);
  color: var(--text-secondary);
  flex-shrink: 0;
}

.btn-remove:hover {
  background: var(--surface);
  color: var(--danger, #ef4444);
}

/* Upload actions */
.upload-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 20px;
}

.upload-hint {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.btn-lg {
  padding: 10px 24px;
  font-size: 0.9375rem;
}

/* Error */
.error-card {
  color: var(--danger, #ef4444);
  margin-top: 16px;
}

/* Results */
.results-summary {
  margin-top: 16px;
}

.results-summary h2 {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 16px;
}

.summary-stats {
  display: flex;
  gap: 24px;
}

.summary-stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.stat-num {
  font-size: 2rem;
  font-weight: 700;
  line-height: 1;
}

.stat-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.imported .stat-num { color: var(--success, #22c55e); }
.duplicate .stat-num { color: var(--warning, #f59e0b); }
.errored .stat-num { color: var(--danger, #ef4444); }

.results-table {
  margin-top: 16px;
  overflow-x: auto;
}

.results-table table {
  width: 100%;
  border-collapse: collapse;
}

.results-table th {
  padding: 10px 16px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.results-table td {
  padding: 10px 16px;
  font-size: 0.875rem;
  border-bottom: 1px solid var(--border);
}

.results-table tr:last-child td {
  border-bottom: none;
}

.name-cell {
  max-width: 400px;
}

.name-cell span:first-child {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-message {
  display: block;
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

.status-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
}

.status-imported {
  background: color-mix(in srgb, var(--success, #22c55e) 15%, transparent);
  color: var(--success, #22c55e);
}

.status-duplicate {
  background: color-mix(in srgb, var(--warning, #f59e0b) 15%, transparent);
  color: var(--warning, #f59e0b);
}

.status-error {
  background: color-mix(in srgb, var(--danger, #ef4444) 15%, transparent);
  color: var(--danger, #ef4444);
}

.private-flag {
  color: var(--warning, #f59e0b);
  font-weight: 600;
  font-size: 0.8125rem;
}

.row-error {
  opacity: 0.7;
}

.results-actions {
  display: flex;
  gap: 12px;
  margin-top: 20px;
}

@media (max-width: 768px) {
  .summary-stats {
    gap: 16px;
  }

  .name-cell {
    max-width: 200px;
  }
}
</style>
