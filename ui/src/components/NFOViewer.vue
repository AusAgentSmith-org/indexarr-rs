<script setup lang="ts">
defineProps<{
  nfo: string
}>()
</script>

<template>
  <div class="nfo-viewer">
    <pre class="nfo-content">{{ nfo }}</pre>
  </div>
</template>

<style scoped>
.nfo-viewer {
  border-radius: var(--radius-md);
  overflow: auto;
  max-height: 500px;
}

.nfo-content {
  background: #1a1a2e;
  color: #d4d4d4;
  padding: 16px;
  margin: 0;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  line-height: 1.4;
  white-space: pre;
  overflow-x: auto;
  border-radius: var(--radius-md);
}
</style>
