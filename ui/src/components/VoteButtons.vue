<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getVotes, castVote } from '@/api'
import type { VoteSummary } from '@/api/types'

const props = defineProps<{
  infoHash: string
}>()

const votes = ref<VoteSummary | null>(null)
const loading = ref(false)

async function load() {
  try {
    votes.value = await getVotes(props.infoHash)
  } catch {
    // silent
  }
}

async function vote(value: 1 | -1) {
  if (loading.value) return
  loading.value = true
  try {
    votes.value = await castVote(props.infoHash, value)
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="vote-buttons" v-if="votes">
    <button
      class="vote-btn upvote"
      :class="{ active: votes.user_vote === 1 }"
      @click="vote(1)"
      :disabled="loading"
      title="Upvote"
    >
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="18 15 12 9 6 15" />
      </svg>
    </button>
    <span class="vote-score" :class="{ positive: votes.score > 0, negative: votes.score < 0 }">
      {{ votes.score }}
    </span>
    <button
      class="vote-btn downvote"
      :class="{ active: votes.user_vote === -1 }"
      @click="vote(-1)"
      :disabled="loading"
      title="Downvote"
    >
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.vote-buttons {
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 2px;
}

.vote-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 28px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: all 0.15s ease;
}

.vote-btn:hover:not(:disabled) {
  background: var(--card);
}

.vote-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.upvote.active {
  color: var(--success);
}

.downvote.active {
  color: var(--accent);
}

.vote-score {
  font-size: 0.875rem;
  font-weight: 700;
  min-width: 24px;
  text-align: center;
  color: var(--text-secondary);
}

.vote-score.positive {
  color: var(--success);
}

.vote-score.negative {
  color: var(--accent);
}
</style>
