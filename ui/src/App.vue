<script setup lang="ts">
import { onMounted } from 'vue'
import AppLayout from '@/components/AppLayout.vue'
import FirstBoot from '@/components/FirstBoot.vue'
import { useIdentityStore } from '@/stores/identity'

const identity = useIdentityStore()

onMounted(() => {
  identity.check()
})
</script>

<template>
  <!-- Loading: blank screen while checking identity -->
  <div v-if="identity.loading" class="app-loading"></div>

  <!-- First boot: show onboarding wizard -->
  <FirstBoot v-else-if="identity.needsOnboarding" />

  <!-- Normal app -->
  <AppLayout v-else>
    <router-view />
  </AppLayout>
</template>

<style scoped>
.app-loading {
  min-height: 100vh;
  background: var(--bg);
}
</style>
