<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  text: string
  truncate?: boolean
}>()

const copied = ref(false)
let timeout: ReturnType<typeof setTimeout> | null = null

async function copy() {
  try {
    await navigator.clipboard.writeText(props.text)
  } catch {
    const textarea = document.createElement('textarea')
    textarea.value = props.text
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
  }
  copied.value = true
  if (timeout) clearTimeout(timeout)
  timeout = setTimeout(() => {
    copied.value = false
  }, 2000)
}
</script>

<template>
  <span class="copy-text" @click="copy" :title="copied ? 'Copied!' : 'Click to copy'">
    <code :class="{ truncated: truncate }">{{ text }}</code>
    <svg v-if="!copied" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" class="copy-icon">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
    <svg v-else viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" class="check-icon">
      <polyline points="20 6 9 17 4 12" />
    </svg>
  </span>
</template>

<style scoped>
.copy-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
}

.copy-text:hover {
  background: var(--surface);
}

code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  color: var(--text);
}

.truncated {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.check-icon {
  color: var(--success);
  flex-shrink: 0;
}
</style>
