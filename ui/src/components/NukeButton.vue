<script setup lang="ts">
import { ref } from 'vue'
import { suggestNuke } from '@/api'

const props = defineProps<{
  infoHash: string
}>()

const showModal = ref(false)
const reason = ref('')
const submitting = ref(false)
const submitted = ref(false)
const error = ref<string | null>(null)

async function submit() {
  if (!reason.value.trim() || reason.value.length < 10) {
    error.value = 'Reason must be at least 10 characters'
    return
  }
  submitting.value = true
  error.value = null
  try {
    await suggestNuke(props.infoHash, reason.value)
    submitted.value = true
    showModal.value = false
    reason.value = ''
  } catch (err: any) {
    if (err?.status === 409) {
      error.value = 'You have already flagged this torrent'
      submitted.value = true
      showModal.value = false
    } else {
      error.value = 'Failed to submit'
    }
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="nuke-wrapper">
    <button
      class="btn btn-nuke"
      :class="{ submitted }"
      @click="showModal = !showModal"
      :disabled="submitted"
      :title="submitted ? 'Already flagged' : 'Suggest removal'"
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      </svg>
      {{ submitted ? 'Flagged' : 'Flag' }}
    </button>

    <!-- Modal -->
    <div v-if="showModal" class="nuke-modal-overlay" @click.self="showModal = false">
      <div class="nuke-modal card-padded">
        <h3>Suggest Removal</h3>
        <p class="nuke-desc">Explain why this torrent should be removed.</p>
        <textarea
          v-model="reason"
          placeholder="Reason for removal (min 10 characters)..."
          rows="3"
          class="nuke-textarea"
        ></textarea>
        <p v-if="error" class="nuke-error">{{ error }}</p>
        <div class="nuke-actions">
          <button class="btn btn-ghost" @click="showModal = false">Cancel</button>
          <button class="btn btn-accent" @click="submit" :disabled="submitting">
            {{ submitting ? 'Submitting...' : 'Submit' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.nuke-wrapper {
  position: relative;
}

.btn-nuke {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: var(--radius);
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--surface);
  border: 1px solid var(--border);
  transition: all 0.15s ease;
}

.btn-nuke:hover:not(:disabled) {
  color: var(--warning);
  border-color: var(--warning);
}

.btn-nuke.submitted {
  opacity: 0.6;
  cursor: not-allowed;
}

.nuke-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.nuke-modal {
  width: 400px;
  max-width: 90vw;
}

.nuke-modal h3 {
  font-size: 1rem;
  font-weight: 600;
  margin-bottom: 8px;
}

.nuke-desc {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.nuke-textarea {
  width: 100%;
  font-family: inherit;
  font-size: 0.875rem;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  color: var(--text);
  resize: vertical;
  outline: none;
}

.nuke-textarea:focus {
  border-color: var(--accent);
}

.nuke-error {
  font-size: 0.8125rem;
  color: var(--accent);
  margin-top: 8px;
}

.nuke-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
