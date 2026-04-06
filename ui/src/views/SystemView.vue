<script setup lang="ts">
import { useRoute } from 'vue-router'

const route = useRoute()

const tabs = [
  { path: '/system/status', label: 'Status', name: 'system-status' },
  { path: '/system/general', label: 'General', name: 'system-general' },
  { path: '/system/sync', label: 'Sync', name: 'system-sync' },
  { path: '/system/api', label: 'API', name: 'system-api' },
  { path: '/system/logs', label: 'Logs', name: 'system-logs' },
]
</script>

<template>
  <div class="system-view">
    <div class="container">
      <h1 class="page-title">System</h1>
      <nav class="system-tabs">
        <router-link
          v-for="tab in tabs"
          :key="tab.path"
          :to="tab.path"
          class="system-tab"
          :class="{ active: route.path === tab.path }"
        >
          {{ tab.label }}
        </router-link>
      </nav>
      <div class="system-content">
        <router-view />
      </div>
    </div>
  </div>
</template>

<style scoped>
.system-view {
  padding-bottom: 40px;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 700;
  padding: 24px 0 0;
}

.system-tabs {
  display: flex;
  gap: 0;
  border-bottom: 2px solid var(--border);
  margin: 16px 0 24px;
}

.system-tab {
  padding: 10px 20px;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-secondary);
  text-decoration: none;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  transition: all 0.15s ease;
}

.system-tab:hover {
  color: var(--text);
  text-decoration: none;
}

.system-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.system-content {
  min-height: 400px;
}
</style>
