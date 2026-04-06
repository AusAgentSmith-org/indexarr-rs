<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getComments, postComment, deleteComment } from '@/api'
import type { Comment } from '@/api/types'
import CommentNode from '@/components/CommentNode.vue'
import Spinner from '@/components/Spinner.vue'

const props = defineProps<{
  infoHash: string
}>()

const comments = ref<Comment[]>([])
const total = ref(0)
const loading = ref(true)
const posting = ref(false)
const error = ref<string | null>(null)

// New comment form
const newBody = ref('')
const replyingTo = ref<number | null>(null)

async function load() {
  loading.value = true
  try {
    const response = await getComments(props.infoHash)
    comments.value = response.comments
    total.value = response.total
  } catch {
    // silent
  } finally {
    loading.value = false
  }
}

async function submitComment() {
  if (!newBody.value.trim()) return
  posting.value = true
  error.value = null
  try {
    await postComment(
      props.infoHash,
      newBody.value,
      replyingTo.value ?? undefined,
    )
    newBody.value = ''
    replyingTo.value = null
    await load()
  } catch (err: any) {
    error.value = err?.message || 'Failed to post comment'
  } finally {
    posting.value = false
  }
}

function handleReply(parentId: number) {
  replyingTo.value = parentId
  // Focus the textarea
  const el = document.querySelector('.comment-textarea') as HTMLTextAreaElement
  if (el) el.focus()
}

async function handleDelete(commentId: number) {
  if (!confirm('Delete this comment?')) return
  try {
    await deleteComment(props.infoHash, commentId)
    await load()
  } catch {
    // silent
  }
}

function cancelReply() {
  replyingTo.value = null
}

function findCommentNickname(id: number, list: Comment[]): string {
  for (const c of list) {
    if (c.id === id) return c.nickname
    if (c.replies.length) {
      const found = findCommentNickname(id, c.replies)
      if (found) return found
    }
  }
  return ''
}

onMounted(load)
</script>

<template>
  <div class="comment-section">
    <div class="card-padded">
      <h3 class="section-subtitle">Comments ({{ total }})</h3>

      <!-- New comment form -->
      <div class="comment-form">
        <div v-if="replyingTo !== null" class="reply-indicator">
          Replying to {{ findCommentNickname(replyingTo, comments) || 'comment' }}
          <button @click="cancelReply" class="cancel-reply">Cancel</button>
        </div>
        <textarea
          v-model="newBody"
          placeholder="Write a comment..."
          rows="3"
          class="comment-textarea"
          @keydown.ctrl.enter="submitComment"
        ></textarea>
        <div class="form-actions">
          <span class="form-hint">Ctrl+Enter to submit</span>
          <button
            class="btn btn-accent"
            @click="submitComment"
            :disabled="posting || !newBody.trim()"
          >
            {{ posting ? 'Posting...' : replyingTo !== null ? 'Reply' : 'Comment' }}
          </button>
        </div>
        <p v-if="error" class="comment-error">{{ error }}</p>
      </div>

      <!-- Comments list -->
      <div v-if="loading" class="loading-state">
        <Spinner :size="24" />
      </div>
      <div v-else-if="comments.length === 0" class="no-comments">
        No comments yet. Be the first!
      </div>
      <div v-else class="comments-list">
        <CommentNode
          v-for="comment in comments"
          :key="comment.id"
          :comment="comment"
          :info-hash="infoHash"
          @reply="handleReply"
          @delete="handleDelete"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.section-subtitle {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 16px;
}

.comment-form {
  margin-bottom: 24px;
  padding-bottom: 20px;
  border-bottom: 1px solid var(--border);
}

.reply-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8125rem;
  color: var(--info);
  margin-bottom: 8px;
  padding: 6px 10px;
  background: var(--surface);
  border-radius: var(--radius-sm);
}

.cancel-reply {
  font-size: 0.75rem;
  color: var(--text-secondary);
  cursor: pointer;
}

.cancel-reply:hover {
  color: var(--accent);
}

.comment-textarea {
  width: 100%;
  font-family: inherit;
  font-size: 0.875rem;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  resize: vertical;
  outline: none;
  min-height: 60px;
}

.comment-textarea:focus {
  border-color: var(--accent);
}

.form-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.form-hint {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.comment-error {
  font-size: 0.8125rem;
  color: var(--accent);
  margin-top: 8px;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 20px 0;
}

.no-comments {
  text-align: center;
  color: var(--text-secondary);
  padding: 20px 0;
  font-size: 0.875rem;
}

.comments-list {
  display: flex;
  flex-direction: column;
}
</style>
