<script setup lang="ts">
defineProps<{
  size?: number
}>()
</script>

<template>
  <div class="spinner" :style="{ width: `${size || 24}px`, height: `${size || 24}px` }"></div>
</template>

<style scoped>
.spinner {
  border: 2.5px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
