<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  currentPage: number
  totalPages: number
  total: number
}>()

const emit = defineEmits<{
  page: [page: number]
}>()

const pages = computed(() => {
  const result: (number | '...')[] = []
  const total = props.totalPages
  const current = props.currentPage

  if (total <= 7) {
    for (let i = 1; i <= total; i++) result.push(i)
    return result
  }

  result.push(1)

  if (current > 3) {
    result.push('...')
  }

  const start = Math.max(2, current - 1)
  const end = Math.min(total - 1, current + 1)

  for (let i = start; i <= end; i++) {
    result.push(i)
  }

  if (current < total - 2) {
    result.push('...')
  }

  result.push(total)

  return result
})
</script>

<template>
  <div class="pagination" v-if="totalPages > 1">
    <span class="page-info">{{ total.toLocaleString() }} results</span>

    <div class="page-controls">
      <button
        class="page-btn"
        :disabled="currentPage <= 1"
        @click="emit('page', currentPage - 1)"
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>

      <template v-for="(p, i) in pages" :key="i">
        <span v-if="p === '...'" class="page-ellipsis">...</span>
        <button
          v-else
          class="page-btn page-num"
          :class="{ active: p === currentPage }"
          @click="emit('page', p)"
        >
          {{ p }}
        </button>
      </template>

      <button
        class="page-btn"
        :disabled="currentPage >= totalPages"
        @click="emit('page', currentPage + 1)"
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
}

.page-info {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.page-controls {
  display: flex;
  align-items: center;
  gap: 4px;
}

.page-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  height: 32px;
  padding: 0 6px;
  border-radius: var(--radius);
  font-size: 0.8125rem;
  color: var(--text-secondary);
  transition: all 0.15s ease;
}

.page-btn:hover:not(:disabled) {
  background: var(--surface);
  color: var(--text);
}

.page-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.page-num.active {
  background: var(--accent);
  color: #fff;
}

.page-ellipsis {
  padding: 0 4px;
  color: var(--text-secondary);
  font-size: 0.8125rem;
}
</style>
