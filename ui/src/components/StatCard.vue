<script setup lang="ts">
import { useFormatters } from '@/composables/useFormatters'

defineProps<{
  label: string
  value: number
  icon?: string
  color?: string
}>()

const { formatNumber } = useFormatters()
</script>

<template>
  <div class="stat-card card-padded">
    <div class="stat-icon" v-if="icon" :style="{ color: color || 'var(--accent)' }">
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
        <path :d="icon" />
      </svg>
    </div>
    <div class="stat-content">
      <span class="stat-value">{{ formatNumber(value) }}</span>
      <span class="stat-label">{{ label }}</span>
    </div>
  </div>
</template>

<style scoped>
.stat-card {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--surface);
}

.stat-content {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 1.5rem;
  font-weight: 700;
  line-height: 1.2;
  color: var(--text);
}

.stat-label {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  margin-top: 2px;
}
</style>
