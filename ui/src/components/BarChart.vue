<script setup lang="ts">
import { computed } from 'vue'

interface BarItem {
  label: string
  value: number
  color?: string
}

const props = defineProps<{
  items: BarItem[]
  title?: string
}>()

const maxValue = computed(() => {
  return Math.max(...props.items.map(i => i.value), 1)
})

const sortedItems = computed(() => {
  return [...props.items].sort((a, b) => b.value - a.value)
})

function formatValue(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toLocaleString()
}
</script>

<template>
  <div class="bar-chart">
    <h4 class="chart-title" v-if="title">{{ title }}</h4>
    <div class="chart-bars">
      <div
        v-for="item in sortedItems"
        :key="item.label"
        class="bar-row"
      >
        <span class="bar-label">{{ item.label }}</span>
        <div class="bar-track">
          <div
            class="bar-fill"
            :style="{
              width: `${(item.value / maxValue) * 100}%`,
              backgroundColor: item.color || 'var(--accent)',
            }"
          ></div>
        </div>
        <span class="bar-value">{{ formatValue(item.value) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.bar-chart {
  /* container */
}

.chart-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 12px;
}

.chart-bars {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bar-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bar-label {
  width: 100px;
  flex-shrink: 0;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  text-align: right;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bar-track {
  flex: 1;
  height: 24px;
  background: var(--surface);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  border-radius: var(--radius-sm);
  transition: width 0.4s ease;
  min-width: 2px;
}

.bar-value {
  width: 60px;
  flex-shrink: 0;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text);
  font-family: var(--font-mono);
}
</style>
