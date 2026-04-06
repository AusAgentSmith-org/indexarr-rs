<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  infoHash: string
  name?: string
}>()

const copied = ref(false)
let timeout: ReturnType<typeof setTimeout> | null = null

function getMagnetUri(): string {
  const dn = props.name ? `&dn=${encodeURIComponent(props.name)}` : ''
  return `magnet:?xt=urn:btih:${props.infoHash}${dn}&tr=udp://tracker.opentrackr.org:1337/announce`
}

async function copyMagnet() {
  try {
    await navigator.clipboard.writeText(getMagnetUri())
    copied.value = true
    if (timeout) clearTimeout(timeout)
    timeout = setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    // Fallback for older browsers
    const textarea = document.createElement('textarea')
    textarea.value = getMagnetUri()
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
    copied.value = true
    if (timeout) clearTimeout(timeout)
    timeout = setTimeout(() => {
      copied.value = false
    }, 2000)
  }
}
</script>

<template>
  <button class="magnet-btn" @click.stop="copyMagnet" :title="copied ? 'Copied!' : 'Copy magnet link'">
    <svg v-if="!copied" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M15 7h3a5 5 0 0 1 5 5 5 5 0 0 1-5 5h-3m-6 0H6a5 5 0 0 1-5-5 5 5 0 0 1 5-5h3" />
      <line x1="8" y1="12" x2="16" y2="12" />
    </svg>
    <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="check-icon">
      <polyline points="20 6 9 17 4 12" />
    </svg>
    <span v-if="copied" class="copied-tooltip">Copied!</span>
  </button>
</template>

<style scoped>
.magnet-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius);
  color: var(--text-secondary);
  transition: all 0.15s ease;
}

.magnet-btn:hover {
  background: var(--surface);
  color: var(--accent);
}

.check-icon {
  color: var(--success);
}

.copied-tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  background: var(--primary);
  color: #fff;
  font-size: 0.6875rem;
  white-space: nowrap;
  pointer-events: none;
  margin-bottom: 4px;
}
</style>
