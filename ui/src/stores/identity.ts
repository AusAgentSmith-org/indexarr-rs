import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getIdentityStatus, restoreIdentity, acknowledgeIdentity } from '@/api'

export const useIdentityStore = defineStore('identity', () => {
  const loading = ref(true)
  const needsOnboarding = ref(false)
  const contributorId = ref<string | null>(null)
  const publicKey = ref<string | null>(null)
  const recoveryKey = ref<string | null>(null)
  const error = ref<string | null>(null)

  async function check() {
    loading.value = true
    error.value = null
    try {
      const status = await getIdentityStatus()
      contributorId.value = status.contributor_id
      publicKey.value = status.public_key
      // Preserve locally cached recovery key if server returns null
      // (server only returns it during onboarding)
      if (status.recovery_key) {
        recoveryKey.value = status.recovery_key
      }
      needsOnboarding.value = status.needs_onboarding
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to check identity'
    } finally {
      loading.value = false
    }
  }

  async function acknowledge() {
    error.value = null
    try {
      const status = await acknowledgeIdentity()
      needsOnboarding.value = false
      recoveryKey.value = null
      contributorId.value = status.contributor_id
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to acknowledge'
    }
  }

  async function restore(key: string) {
    error.value = null
    try {
      const status = await restoreIdentity(key)
      contributorId.value = status.contributor_id
      publicKey.value = status.public_key
      recoveryKey.value = null
      needsOnboarding.value = false
      return true
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Invalid recovery key'
      return false
    }
  }

  return {
    loading,
    needsOnboarding,
    contributorId,
    publicKey,
    recoveryKey,
    error,
    check,
    acknowledge,
    restore,
  }
})
