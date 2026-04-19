<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  search: [value: string]
}>()

const input = ref(props.modelValue)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

watch(() => props.modelValue, (val) => {
  input.value = val
})

function onInput() {
  emit('update:modelValue', input.value)
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    emit('search', input.value)
  }, 350)
}

function onSubmit() {
  if (debounceTimer) clearTimeout(debounceTimer)
  emit('search', input.value)
}

function clear() {
  input.value = ''
  emit('update:modelValue', '')
  emit('search', '')
}

const inputRef = ref<HTMLInputElement>()

onMounted(() => {
  inputRef.value?.focus()
})
</script>

<template>
  <form class="search-bar" @submit.prevent="onSubmit">
    <div class="search-input-wrapper">
      <svg class="search-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
      </svg>
      <input
        ref="inputRef"
        v-model="input"
        type="search"
        class="search-input"
        placeholder="Search torrents..."
        @input="onInput"
        autocomplete="off"
        spellcheck="false"
      />
      <button
        v-if="input"
        type="button"
        class="clear-btn"
        @click="clear"
        title="Clear search"
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </form>
</template>

<style scoped>
.search-bar {
  width: 100%;
  max-width: 720px;
}

.search-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 14px;
  color: var(--text-secondary);
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 12px 40px 12px 44px;
  font-size: 1rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--card);
  color: var(--text);
  outline: none;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.search-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(233, 69, 96, 0.1);
}

.search-input::placeholder {
  color: var(--text-secondary);
}

/* Remove native search clear button */
.search-input::-webkit-search-cancel-button {
  display: none;
}

.clear-btn {
  position: absolute;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: all 0.15s ease;
}

.clear-btn:hover {
  background: var(--surface);
  color: var(--text);
}
</style>
