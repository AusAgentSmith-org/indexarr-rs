<script setup lang="ts">
import { computed, ref } from 'vue'
import { useFormatters } from '@/composables/useFormatters'
import type { TorrentFile } from '@/api'

const props = defineProps<{
  files: TorrentFile[]
}>()

const { formatBytes } = useFormatters()

interface TreeNode {
  name: string
  path: string
  size: number
  isDir: boolean
  children: TreeNode[]
  extension: string | null
}

const tree = computed(() => {
  const root: TreeNode[] = []

  for (const file of props.files) {
    const parts = file.path.split('/')
    let current = root

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      const isLast = i === parts.length - 1

      let existing = current.find(n => n.name === part)
      if (!existing) {
        existing = {
          name: part,
          path: parts.slice(0, i + 1).join('/'),
          size: isLast ? file.size : 0,
          isDir: !isLast,
          children: [],
          extension: isLast ? file.extension : null,
        }
        current.push(existing)
      }
      if (!isLast) {
        existing.isDir = true
        current = existing.children
      }
    }
  }

  // Calculate directory sizes
  function calcSize(nodes: TreeNode[]): number {
    let total = 0
    for (const node of nodes) {
      if (node.isDir) {
        node.size = calcSize(node.children)
      }
      total += node.size
    }
    return total
  }
  calcSize(root)

  // Sort: dirs first, then files alphabetically
  function sortNodes(nodes: TreeNode[]) {
    nodes.sort((a, b) => {
      if (a.isDir && !b.isDir) return -1
      if (!a.isDir && b.isDir) return 1
      return a.name.localeCompare(b.name)
    })
    for (const node of nodes) {
      if (node.children.length) sortNodes(node.children)
    }
  }
  sortNodes(root)

  return root
})

const collapsed = ref<Set<string>>(new Set())

function toggleDir(path: string) {
  if (collapsed.value.has(path)) {
    collapsed.value.delete(path)
  } else {
    collapsed.value.add(path)
  }
}

function isCollapsed(path: string): boolean {
  return collapsed.value.has(path)
}

// File icon helper - available for template use if needed
// function getFileIcon(ext: string | null): string { ... }
</script>

<template>
  <div class="file-tree">
    <template v-for="node in tree" :key="node.path">
      <div
        class="tree-node"
        :class="{ directory: node.isDir }"
      >
        <div class="node-row" @click="node.isDir && toggleDir(node.path)">
          <!-- Directory -->
          <template v-if="node.isDir">
            <svg class="node-icon dir-icon" :class="{ collapsed: isCollapsed(node.path) }" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="6 9 12 15 18 9" />
            </svg>
            <svg class="node-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
          </template>
          <!-- File -->
          <template v-else>
            <span class="node-spacer"></span>
            <svg class="node-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" />
            </svg>
          </template>
          <span class="node-name" :class="{ 'is-dir': node.isDir }">{{ node.name }}</span>
          <span class="node-size">{{ formatBytes(node.size) }}</span>
        </div>
        <!-- Children -->
        <div v-if="node.isDir && !isCollapsed(node.path)" class="node-children">
          <FileTree :files="node.children.filter(c => !c.isDir).map(c => ({ path: c.name, size: c.size, extension: c.extension }))" v-if="node.children.some(c => !c.isDir) && node.children.every(c => !c.isDir)" />
          <template v-else>
            <div
              v-for="child in node.children"
              :key="child.path"
              class="tree-node"
              :class="{ directory: child.isDir }"
            >
              <div class="node-row" @click="child.isDir && toggleDir(child.path)">
                <template v-if="child.isDir">
                  <svg class="node-icon dir-icon" :class="{ collapsed: isCollapsed(child.path) }" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="6 9 12 15 18 9" />
                  </svg>
                  <svg class="node-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                  </svg>
                </template>
                <template v-else>
                  <span class="node-spacer"></span>
                  <svg class="node-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" />
                  </svg>
                </template>
                <span class="node-name" :class="{ 'is-dir': child.isDir }">{{ child.name }}</span>
                <span class="node-size">{{ formatBytes(child.size) }}</span>
              </div>
              <!-- Recursive children for nested dirs -->
              <div v-if="child.isDir && !isCollapsed(child.path)" class="node-children">
                <FileTree :files="child.children.map(c => ({ path: c.path.split('/').slice(1).join('/') || c.name, size: c.size, extension: c.extension }))" />
              </div>
            </div>
          </template>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.file-tree {
  font-size: 0.8125rem;
}

.tree-node {
  /* nested structure */
}

.node-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: default;
}

.directory > .node-row {
  cursor: pointer;
}

.node-row:hover {
  background: var(--surface);
}

.node-icon {
  flex-shrink: 0;
  color: var(--text-secondary);
}

.dir-icon {
  transition: transform 0.15s ease;
}

.dir-icon.collapsed {
  transform: rotate(-90deg);
}

.node-spacer {
  width: 14px;
  flex-shrink: 0;
}

.node-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-name.is-dir {
  font-weight: 600;
}

.node-size {
  flex-shrink: 0;
  color: var(--text-secondary);
  font-size: 0.75rem;
  font-family: var(--font-mono);
}

.node-children {
  padding-left: 20px;
}
</style>
