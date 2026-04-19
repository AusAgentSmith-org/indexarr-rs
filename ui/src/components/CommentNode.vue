<script setup lang="ts">
import { ref } from 'vue'
import type { Comment } from '@/api/types'
import { useFormatters } from '@/composables/useFormatters'

const props = defineProps<{
  comment: Comment
  infoHash: string
  depth?: number
}>()

const emit = defineEmits<{
  reply: [parentId: number]
  delete: [commentId: number]
}>()

const { formatDate } = useFormatters()
const maxDepth = 5
const currentDepth = props.depth ?? 0
</script>

<template>
  <div class="comment-node" :class="{ 'is-deleted': comment.deleted }">
    <div class="comment-header">
      <span class="comment-author">{{ comment.nickname }}</span>
      <span class="comment-date">{{ formatDate(comment.created_at) }}</span>
    </div>
    <div class="comment-body">{{ comment.body }}</div>
    <div class="comment-actions" v-if="!comment.deleted">
      <button class="comment-action" @click="emit('reply', comment.id)" v-if="currentDepth < maxDepth">
        Reply
      </button>
      <button class="comment-action delete-action" @click="emit('delete', comment.id)" v-if="comment.is_own">
        Delete
      </button>
    </div>
    <div class="comment-replies" v-if="comment.replies && comment.replies.length > 0">
      <CommentNode
        v-for="reply in comment.replies"
        :key="reply.id"
        :comment="reply"
        :info-hash="infoHash"
        :depth="currentDepth + 1"
        @reply="emit('reply', $event)"
        @delete="emit('delete', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.comment-node {
  padding: 12px 0 8px;
  border-left: 2px solid var(--border);
  padding-left: 16px;
  margin-left: 0;
}

.comment-node:first-child {
  padding-top: 0;
}

.comment-node.is-deleted {
  opacity: 0.5;
}

.comment-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.comment-author {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text);
}

.comment-date {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.comment-body {
  font-size: 0.875rem;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.comment-actions {
  display: flex;
  gap: 12px;
  margin-top: 4px;
}

.comment-action {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 2px 0;
}

.comment-action:hover {
  color: var(--accent);
}

.delete-action:hover {
  color: var(--accent);
}

.comment-replies {
  margin-top: 8px;
  margin-left: 4px;
}
</style>
