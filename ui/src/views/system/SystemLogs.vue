<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch, computed } from 'vue'
import { getRecentLogs } from '@/api'
import type { LogEntry } from '@/api/types'
import Spinner from '@/components/Spinner.vue'

const entries = ref<LogEntry[]>([])
const categories = ref<string[]>([])
const selectedCategory = ref<string | null>(null)
const debugEnabled = ref(false)
const autoScroll = ref(true)
const loading = ref(true)
const connected = ref(false)

let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let unmounted = false
const logContainer = ref<HTMLElement | null>(null)

const filteredEntries = computed(() => {
  let list = entries.value
  if (selectedCategory.value) {
    list = list.filter(e => e.category === selectedCategory.value)
  }
  if (!debugEnabled.value) {
    list = list.filter(e => e.level !== 'debug')
  }
  return list
})

function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function levelClass(level: string): string {
  switch (level) {
    case 'error': return 'log-error'
    case 'warning': return 'log-warning'
    case 'debug': return 'log-debug'
    default: return 'log-info'
  }
}

function scrollToBottom() {
  if (autoScroll.value && logContainer.value) {
    nextTick(() => {
      if (logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight
      }
    })
  }
}

async function loadInitial() {
  loading.value = true
  try {
    const response = await getRecentLogs(500)
    entries.value = response.entries
    categories.value = response.categories
    debugEnabled.value = response.debug_enabled
    scrollToBottom()
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

function connectWebSocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const wsUrl = `${protocol}//${window.location.host}/api/v1/system/logs/ws`

  ws = new WebSocket(wsUrl)

  ws.onopen = () => {
    connected.value = true
  }

  ws.onmessage = (event) => {
    try {
      const entry: LogEntry = JSON.parse(event.data)
      entries.value.push(entry)

      // Keep buffer manageable
      if (entries.value.length > 5000) {
        entries.value = entries.value.slice(-3000)
      }

      // Update categories
      if (!categories.value.includes(entry.category)) {
        categories.value = [...categories.value, entry.category].sort()
      }

      scrollToBottom()
    } catch {
      // ignore parse errors
    }
  }

  ws.onclose = () => {
    connected.value = false
    if (!unmounted) {
      reconnectTimer = setTimeout(connectWebSocket, 3000)
    }
  }

  ws.onerror = () => {
    connected.value = false
  }
}

async function toggleDebug() {
  debugEnabled.value = !debugEnabled.value
  try {
    const { apiFetch } = await import('@/api/client')
    await apiFetch('/system/logs/debug', {
      method: 'POST',
      body: JSON.stringify({ enabled: debugEnabled.value }),
    })
  } catch {
    debugEnabled.value = !debugEnabled.value
  }
}

function clearLogs() {
  entries.value = []
}

onMounted(() => {
  loadInitial()
  connectWebSocket()
})

onUnmounted(() => {
  unmounted = true
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (ws) {
    ws.close()
    ws = null
  }
})

watch(selectedCategory, () => {
  scrollToBottom()
})
</script>

<template>
  <div class="logs-page">
    <!-- Controls -->
    <div class="logs-controls">
      <div class="controls-left">
        <!-- Category filter -->
        <select v-model="selectedCategory" class="category-select">
          <option :value="null">All Categories</option>
          <option v-for="cat in categories" :key="cat" :value="cat">{{ cat }}</option>
        </select>

        <!-- Connection status -->
        <span class="connection-status" :class="{ online: connected }">
          <span class="status-dot"></span>
          {{ connected ? 'Live' : 'Disconnected' }}
        </span>
      </div>

      <div class="controls-right">
        <!-- Debug toggle -->
        <label class="toggle-label">
          <input type="checkbox" :checked="debugEnabled" @change="toggleDebug" class="toggle-input" />
          <span class="toggle-switch"></span>
          Debug
        </label>

        <!-- Auto-scroll toggle -->
        <label class="toggle-label">
          <input type="checkbox" v-model="autoScroll" class="toggle-input" />
          <span class="toggle-switch"></span>
          Auto-scroll
        </label>

        <button class="btn btn-ghost" @click="clearLogs">Clear</button>
      </div>
    </div>

    <!-- Log output -->
    <div class="card log-card">
      <div v-if="loading" class="loading-state">
        <Spinner :size="24" />
      </div>
      <div v-else ref="logContainer" class="log-output">
        <div v-if="filteredEntries.length === 0" class="log-empty">
          No log entries{{ selectedCategory ? ` for "${selectedCategory}"` : '' }}
        </div>
        <div
          v-for="(entry, i) in filteredEntries"
          :key="i"
          class="log-line"
          :class="levelClass(entry.level)"
        >
          <span class="log-time">{{ formatTime(entry.timestamp) }}</span>
          <span class="log-category">{{ entry.category }}</span>
          <span class="log-message">{{ entry.message }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.logs-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.logs-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

.controls-left,
.controls-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.category-select {
  font-family: inherit;
  font-size: 0.875rem;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  outline: none;
  cursor: pointer;
}

.category-select:focus {
  border-color: var(--accent);
}

.connection-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-secondary);
}

.connection-status.online .status-dot {
  background: var(--success);
}

.connection-status.online {
  color: var(--success);
}

/* Toggle switch */
.toggle-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}

.toggle-input {
  display: none;
}

.toggle-switch {
  position: relative;
  width: 36px;
  height: 20px;
  background: var(--border);
  border-radius: 10px;
  transition: background 0.2s ease;
}

.toggle-switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  transition: transform 0.2s ease;
}

.toggle-input:checked + .toggle-switch {
  background: var(--accent);
}

.toggle-input:checked + .toggle-switch::after {
  transform: translateX(16px);
}

/* Log output */
.log-card {
  overflow: hidden;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

.log-output {
  height: 600px;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  line-height: 1.6;
  padding: 8px 0;
}

.log-empty {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  color: var(--text-secondary);
  font-family: var(--font-sans);
}

.log-line {
  display: flex;
  gap: 12px;
  padding: 2px 16px;
  white-space: nowrap;
}

.log-line:hover {
  background: var(--surface);
}

.log-time {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.log-category {
  color: var(--info);
  flex-shrink: 0;
  min-width: 100px;
}

.log-message {
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-all;
}

/* Log levels */
.log-error .log-message {
  color: var(--accent);
}

.log-warning .log-message {
  color: var(--warning);
}

.log-debug .log-message {
  color: var(--text-secondary);
}

.log-debug .log-category {
  color: var(--text-secondary);
}

@media (max-width: 768px) {
  .logs-controls {
    flex-direction: column;
    align-items: flex-start;
  }

  .log-output {
    height: 400px;
  }
}
</style>
