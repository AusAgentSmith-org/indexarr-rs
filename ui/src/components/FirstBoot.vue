<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useIdentityStore } from '@/stores/identity'
import { getSyncPreferences, setSyncPreferences } from '@/api'

const identity = useIdentityStore()
const step = ref<'welcome' | 'generating' | 'ready' | 'categories' | 'restore'>('welcome')
const confirmed = ref(false)
const copied = ref(false)
const restoreKey = ref('')
const restoreError = ref('')

// Sync category selection state
const allCategories = ref<string[]>([])
const selectedCategories = ref<string[]>([])
const syncComments = ref(true)
const categoriesLoading = ref(false)
const categoriesSaving = ref(false)

const categoryLabels: Record<string, string> = {
  movie: 'Movies',
  tv_show: 'TV Shows',
  music: 'Music',
  ebook: 'Ebooks',
  comic: 'Comics',
  audiobook: 'Audiobooks',
  game: 'Games',
  software: 'Software',
  xxx: 'XXX',
  unknown: 'Unknown',
}

const categoryIcons: Record<string, string> = {
  movie: '🎬',
  tv_show: '📺',
  music: '🎵',
  ebook: '📚',
  comic: '💬',
  audiobook: '🎧',
  game: '🎮',
  software: '💾',
  xxx: '🔞',
  unknown: '❓',
}

const allSelected = computed(() =>
  selectedCategories.value.length === allCategories.value.length
)

const noneSelected = computed(() =>
  selectedCategories.value.length === 0
)

let copyTimeout: ReturnType<typeof setTimeout> | null = null

async function generate() {
  step.value = 'generating'
  // Identity is already generated server-side; fetch it with a brief animation delay
  await identity.check()
  await new Promise(r => setTimeout(r, 2200))
  step.value = 'ready'
}

async function copyRecoveryKey() {
  if (!identity.recoveryKey) return
  try {
    await navigator.clipboard.writeText(identity.recoveryKey)
  } catch {
    const ta = document.createElement('textarea')
    ta.value = identity.recoveryKey
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }
  copied.value = true
  if (copyTimeout) clearTimeout(copyTimeout)
  copyTimeout = setTimeout(() => { copied.value = false }, 3000)
}

async function doRestore() {
  restoreError.value = ''
  const key = restoreKey.value.trim()
  if (!key) {
    restoreError.value = 'Please enter your recovery key'
    return
  }
  const ok = await identity.restore(key)
  if (!ok) {
    restoreError.value = identity.error || 'Invalid recovery key'
  }
}

async function goToCategories() {
  categoriesLoading.value = true
  step.value = 'categories'
  try {
    const prefs = await getSyncPreferences()
    allCategories.value = prefs.all_categories
    selectedCategories.value = [...prefs.import_categories]
    syncComments.value = prefs.sync_comments ?? true
  } catch {
    // Fall back to showing empty categories
  } finally {
    categoriesLoading.value = false
  }
}

function toggleCategory(cat: string) {
  const idx = selectedCategories.value.indexOf(cat)
  if (idx >= 0) {
    selectedCategories.value = selectedCategories.value.filter(c => c !== cat)
  } else {
    selectedCategories.value = [...selectedCategories.value, cat]
  }
}

function selectAll() {
  selectedCategories.value = [...allCategories.value]
}

function selectNone() {
  selectedCategories.value = []
}

async function enter() {
  categoriesSaving.value = true
  try {
    await setSyncPreferences(selectedCategories.value, syncComments.value)
  } catch {
    // Continue even if save fails — user can change later
  }
  categoriesSaving.value = false
  await identity.acknowledge()
}

onMounted(() => {
  // If we already have recovery key data, skip straight to ready
  if (identity.recoveryKey && identity.contributorId) {
    step.value = 'ready'
  }
})
</script>

<template>
  <div class="first-boot">
    <!-- Ambient background dots -->
    <div class="ambient">
      <div class="dot dot-1"></div>
      <div class="dot dot-2"></div>
      <div class="dot dot-3"></div>
      <div class="dot dot-4"></div>
      <div class="dot dot-5"></div>
    </div>

    <Transition name="step" mode="out-in">
      <!-- WELCOME -->
      <div v-if="step === 'welcome'" key="welcome" class="step-panel">
        <div class="logo-wrap">
          <img class="logo" src="/logo-192.png" alt="Indexarr" />
        </div>

        <h1 class="title">Welcome to Indexarr</h1>
        <p class="subtitle">Decentralized Torrent Indexing</p>

        <p class="description">
          Before you begin, we need to set up your contributor identity.
          This is an anonymous cryptographic keypair that lets you participate
          in the network.
        </p>

        <div class="privacy-box">
          <svg class="privacy-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          <p>
            <strong>Your privacy is protected.</strong>
            No personal information is ever collected or stored.
            Your contributor ID is a random cryptographic identifier
            &mdash; it cannot be traced back to you.
            IDs are only used to protect the swarm against malicious actors.
          </p>
        </div>

        <div class="actions">
          <button class="btn-generate" @click="generate">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
              <path d="M7 11V7a5 5 0 0 1 10 0v4" />
            </svg>
            Generate My Identity
          </button>
          <button class="btn-restore-link" @click="step = 'restore'">
            I have an existing recovery key
          </button>
        </div>
      </div>

      <!-- GENERATING -->
      <div v-else-if="step === 'generating'" key="generating" class="step-panel generating-panel">
        <div class="gen-ring">
          <div class="ring-outer"></div>
          <div class="ring-inner"></div>
        </div>
        <p class="gen-text">Generating cryptographic keypair</p>
        <div class="gen-dots">
          <span></span><span></span><span></span>
        </div>
      </div>

      <!-- IDENTITY READY -->
      <div v-else-if="step === 'ready'" key="ready" class="step-panel ready-panel">
        <div class="success-icon">
          <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
        </div>

        <h2 class="ready-title">Your Identity is Ready</h2>

        <div class="id-card">
          <label>Contributor ID</label>
          <div class="id-value">{{ identity.contributorId }}</div>
        </div>

        <div class="recovery-section">
          <h3>Recovery Key</h3>
          <p class="recovery-warning">
            This is the <strong>only way</strong> to restore your identity on
            another device or after data loss. Save it somewhere safe.
          </p>
          <div class="recovery-box">
            <code class="recovery-key">{{ identity.recoveryKey }}</code>
            <button class="copy-btn" @click="copyRecoveryKey">
              <svg v-if="!copied" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              {{ copied ? 'Copied!' : 'Copy' }}
            </button>
          </div>
        </div>

        <label class="confirm-label" :class="{ checked: confirmed }">
          <input type="checkbox" v-model="confirmed" />
          <span class="checkmark"></span>
          I have safely stored my recovery key
        </label>

        <button class="btn-enter" :disabled="!confirmed" @click="goToCategories">
          Continue
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="5" y1="12" x2="19" y2="12" /><polyline points="12 5 19 12 12 19" />
          </svg>
        </button>
      </div>

      <!-- SYNC CATEGORIES -->
      <div v-else-if="step === 'categories'" key="categories" class="step-panel categories-panel">
        <div class="categories-icon">
          <svg viewBox="0 0 24 24" width="36" height="36" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </div>

        <h2 class="categories-title">Sync Categories</h2>
        <p class="categories-desc">
          Choose which content categories to <strong>sync</strong> from the swarm.
          You can change these later under System &rarr; Sync.
        </p>

        <div v-if="categoriesLoading" class="categories-loading">
          <div class="gen-ring">
            <div class="ring-outer"></div>
            <div class="ring-inner"></div>
          </div>
        </div>

        <template v-else>
          <div class="cat-bulk-actions">
            <button class="btn-cat-bulk" @click="selectAll" :disabled="allSelected">Select All</button>
            <button class="btn-cat-bulk" @click="selectNone" :disabled="noneSelected">Select None</button>
          </div>

          <div class="category-grid-fb">
            <label
              v-for="cat in allCategories"
              :key="cat"
              class="cat-item-fb"
              :class="{ selected: selectedCategories.includes(cat) }"
            >
              <input
                type="checkbox"
                :checked="selectedCategories.includes(cat)"
                @change="toggleCategory(cat)"
                class="cat-checkbox-fb"
              />
              <span class="cat-icon-fb">{{ categoryIcons[cat] || '📦' }}</span>
              <span class="cat-name-fb">{{ categoryLabels[cat] || cat }}</span>
            </label>
          </div>

          <label class="toggle-comments-fb">
            <input type="checkbox" v-model="syncComments" class="toggle-cb-fb" />
            <div class="toggle-info-fb">
              <span class="toggle-label-fb">Sync comments &amp; votes</span>
              <span class="toggle-desc-fb">Import user comments and votes from peers</span>
            </div>
          </label>

          <div v-if="noneSelected" class="cat-warning-fb">
            No categories selected — sync imports will be disabled.
          </div>

          <button class="btn-enter" :disabled="categoriesSaving" @click="enter">
            {{ categoriesSaving ? 'Saving...' : 'Enter Indexarr' }}
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" /><polyline points="12 5 19 12 12 19" />
            </svg>
          </button>
        </template>
      </div>

      <!-- RESTORE -->
      <div v-else-if="step === 'restore'" key="restore" class="step-panel restore-panel">
        <h2 class="restore-title">Restore Identity</h2>
        <p class="restore-desc">
          Enter your recovery key to restore an existing contributor identity.
        </p>

        <div class="restore-input-wrap">
          <input
            v-model="restoreKey"
            type="text"
            class="restore-input"
            placeholder="XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX"
            spellcheck="false"
            autocomplete="off"
            @keydown.enter="doRestore"
          />
        </div>

        <p v-if="restoreError" class="restore-error">{{ restoreError }}</p>

        <div class="restore-actions">
          <button class="btn-do-restore" @click="doRestore" :disabled="!restoreKey.trim()">
            Restore Identity
          </button>
          <button class="btn-back" @click="step = 'welcome'; restoreError = ''">
            Back
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.first-boot {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #0a0a1a;
  background: radial-gradient(ellipse at 50% 30%, #12122e 0%, #0a0a1a 70%);
  color: #e8e8e8;
  font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
  overflow: hidden;
}

/* ---- Ambient floating dots ---- */
.ambient {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}
.dot {
  position: absolute;
  border-radius: 50%;
  background: rgba(233, 69, 96, 0.08);
  animation: float 20s ease-in-out infinite;
}
.dot-1 { width: 300px; height: 300px; top: -100px; left: -80px; animation-delay: 0s; }
.dot-2 { width: 200px; height: 200px; bottom: -60px; right: -40px; animation-delay: -5s; }
.dot-3 { width: 150px; height: 150px; top: 30%; right: 10%; animation-delay: -10s; background: rgba(79, 195, 247, 0.06); }
.dot-4 { width: 100px; height: 100px; bottom: 20%; left: 15%; animation-delay: -15s; background: rgba(78, 204, 163, 0.06); }
.dot-5 { width: 250px; height: 250px; top: 50%; left: 50%; transform: translate(-50%, -50%); animation-delay: -8s; background: rgba(233, 69, 96, 0.04); }

@keyframes float {
  0%, 100% { transform: translate(0, 0) scale(1); }
  25% { transform: translate(30px, -20px) scale(1.05); }
  50% { transform: translate(-20px, 30px) scale(0.95); }
  75% { transform: translate(20px, 20px) scale(1.02); }
}

/* ---- Step transitions ---- */
.step-enter-active,
.step-leave-active {
  transition: all 0.4s ease;
}
.step-enter-from {
  opacity: 0;
  transform: translateY(20px);
}
.step-leave-to {
  opacity: 0;
  transform: translateY(-20px);
}

/* ---- Step panel (shared) ---- */
.step-panel {
  position: relative;
  max-width: 520px;
  width: 90vw;
  padding: 48px 40px;
  background: rgba(22, 33, 62, 0.85);
  border: 1px solid rgba(233, 69, 96, 0.15);
  border-radius: 16px;
  backdrop-filter: blur(20px);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px rgba(233, 69, 96, 0.05);
  text-align: center;
}

/* ---- Welcome step ---- */
.logo-wrap {
  margin-bottom: 24px;
}
.logo {
  width: 64px;
  height: 64px;
  color: #e94560;
  animation: logo-pulse 3s ease-in-out infinite;
}
@keyframes logo-pulse {
  0%, 100% { opacity: 0.8; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.05); }
}

.title {
  font-size: 1.75rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  margin-bottom: 4px;
}
.subtitle {
  font-size: 0.9rem;
  color: #8888aa;
  margin-bottom: 28px;
}
.description {
  font-size: 0.9rem;
  line-height: 1.65;
  color: #bbb;
  margin-bottom: 24px;
}

.privacy-box {
  display: flex;
  gap: 14px;
  text-align: left;
  background: rgba(78, 204, 163, 0.06);
  border: 1px solid rgba(78, 204, 163, 0.15);
  border-radius: 10px;
  padding: 16px 18px;
  margin-bottom: 32px;
}
.privacy-icon {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  color: #4ecca3;
  margin-top: 2px;
}
.privacy-box p {
  font-size: 0.82rem;
  line-height: 1.6;
  color: #aaa;
}
.privacy-box strong {
  color: #4ecca3;
}

.actions {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.btn-generate {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 14px 32px;
  background: #e94560;
  color: #fff;
  font-size: 1rem;
  font-weight: 600;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 4px 20px rgba(233, 69, 96, 0.3);
}
.btn-generate:hover {
  background: #f05672;
  transform: translateY(-1px);
  box-shadow: 0 6px 24px rgba(233, 69, 96, 0.4);
}
.btn-generate:active {
  transform: translateY(0);
}

.btn-restore-link {
  background: none;
  border: none;
  color: #8888aa;
  font-size: 0.85rem;
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 3px;
  transition: color 0.15s ease;
}
.btn-restore-link:hover {
  color: #e8e8e8;
}

/* ---- Generating step ---- */
.generating-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 64px 40px;
}

.gen-ring {
  position: relative;
  width: 80px;
  height: 80px;
  margin-bottom: 32px;
}
.ring-outer {
  position: absolute;
  inset: 0;
  border: 2px solid rgba(233, 69, 96, 0.15);
  border-top-color: #e94560;
  border-radius: 50%;
  animation: spin 1.2s linear infinite;
}
.ring-inner {
  position: absolute;
  inset: 12px;
  border: 2px solid rgba(79, 195, 247, 0.1);
  border-bottom-color: #4fc3f7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite reverse;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.gen-text {
  font-size: 1rem;
  color: #ccc;
  margin-bottom: 8px;
}
.gen-dots {
  display: flex;
  gap: 6px;
}
.gen-dots span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #e94560;
  animation: dot-bounce 1.4s ease-in-out infinite;
}
.gen-dots span:nth-child(2) { animation-delay: 0.2s; }
.gen-dots span:nth-child(3) { animation-delay: 0.4s; }
@keyframes dot-bounce {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.2); }
}

/* ---- Ready step ---- */
.ready-panel {
  max-width: 560px;
}

.success-icon {
  margin-bottom: 16px;
  color: #4ecca3;
  animation: pop-in 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
@keyframes pop-in {
  0% { opacity: 0; transform: scale(0.5); }
  100% { opacity: 1; transform: scale(1); }
}

.ready-title {
  font-size: 1.4rem;
  font-weight: 700;
  margin-bottom: 24px;
}

.id-card {
  background: rgba(233, 69, 96, 0.08);
  border: 1px solid rgba(233, 69, 96, 0.2);
  border-radius: 10px;
  padding: 16px 20px;
  margin-bottom: 28px;
}
.id-card label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: #8888aa;
  display: block;
  margin-bottom: 6px;
}
.id-value {
  font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
  font-size: 1.5rem;
  font-weight: 700;
  color: #e94560;
  letter-spacing: 0.03em;
}

.recovery-section {
  text-align: left;
  margin-bottom: 24px;
}
.recovery-section h3 {
  font-size: 0.95rem;
  font-weight: 600;
  margin-bottom: 8px;
}
.recovery-warning {
  font-size: 0.82rem;
  color: #aaa;
  line-height: 1.5;
  margin-bottom: 12px;
}
.recovery-warning strong {
  color: #f9a825;
}

.recovery-box {
  display: flex;
  align-items: center;
  gap: 10px;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 14px 16px;
}
.recovery-key {
  font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
  font-size: 0.82rem;
  color: #e8e8e8;
  word-break: break-all;
  line-height: 1.6;
  flex: 1;
}

.copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  color: #ccc;
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
  flex-shrink: 0;
}
.copy-btn:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

/* ---- Confirm checkbox ---- */
.confirm-label {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 24px;
  cursor: pointer;
  font-size: 0.9rem;
  color: #aaa;
  transition: color 0.15s ease;
  user-select: none;
}
.confirm-label.checked {
  color: #e8e8e8;
}
.confirm-label input[type="checkbox"] {
  display: none;
}
.checkmark {
  width: 20px;
  height: 20px;
  border: 2px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}
.confirm-label.checked .checkmark {
  background: #e94560;
  border-color: #e94560;
}
.confirm-label.checked .checkmark::after {
  content: '';
  width: 5px;
  height: 10px;
  border: solid #fff;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
  margin-top: -2px;
}

.btn-enter {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 14px 36px;
  background: #e94560;
  color: #fff;
  font-size: 1rem;
  font-weight: 600;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 4px 20px rgba(233, 69, 96, 0.3);
}
.btn-enter:hover:not(:disabled) {
  background: #f05672;
  transform: translateY(-1px);
  box-shadow: 0 6px 24px rgba(233, 69, 96, 0.4);
}
.btn-enter:disabled {
  opacity: 0.35;
  cursor: not-allowed;
  box-shadow: none;
}

/* ---- Restore step ---- */
.restore-panel {
  max-width: 480px;
}
.restore-title {
  font-size: 1.3rem;
  font-weight: 700;
  margin-bottom: 8px;
}
.restore-desc {
  font-size: 0.88rem;
  color: #aaa;
  margin-bottom: 24px;
  line-height: 1.5;
}

.restore-input-wrap {
  margin-bottom: 8px;
}
.restore-input {
  width: 100%;
  padding: 14px 16px;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #e8e8e8;
  font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
  font-size: 0.9rem;
  outline: none;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.restore-input:focus {
  border-color: #e94560;
  box-shadow: 0 0 0 3px rgba(233, 69, 96, 0.15);
}
.restore-input::placeholder {
  color: #555;
}

.restore-error {
  color: #e94560;
  font-size: 0.82rem;
  margin-bottom: 16px;
  text-align: left;
}

.restore-actions {
  display: flex;
  gap: 12px;
  margin-top: 20px;
}
.btn-do-restore {
  flex: 1;
  padding: 12px 24px;
  background: #e94560;
  color: #fff;
  font-size: 0.9rem;
  font-weight: 600;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}
.btn-do-restore:hover:not(:disabled) {
  background: #f05672;
}
.btn-do-restore:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.btn-back {
  padding: 12px 24px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #aaa;
  font-size: 0.9rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn-back:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #e8e8e8;
}

/* ---- Categories step ---- */
.categories-panel {
  max-width: 600px;
}

.categories-icon {
  margin-bottom: 12px;
  color: #4fc3f7;
}

.categories-title {
  font-size: 1.4rem;
  font-weight: 700;
  margin-bottom: 8px;
}

.categories-desc {
  font-size: 0.88rem;
  color: #aaa;
  line-height: 1.5;
  margin-bottom: 24px;
}

.categories-desc strong {
  color: #e8e8e8;
}

.categories-loading {
  display: flex;
  justify-content: center;
  padding: 32px 0;
}

.cat-bulk-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-bottom: 12px;
}

.btn-cat-bulk {
  padding: 4px 12px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #aaa;
  font-size: 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-cat-bulk:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
  color: #e8e8e8;
}

.btn-cat-bulk:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.category-grid-fb {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px;
  margin-bottom: 20px;
}

.cat-item-fb {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}

.cat-item-fb:hover {
  border-color: rgba(255, 255, 255, 0.2);
}

.cat-item-fb.selected {
  border-color: #e94560;
  background: rgba(233, 69, 96, 0.1);
}

.cat-checkbox-fb {
  display: none;
}

.cat-icon-fb {
  font-size: 1.1rem;
  line-height: 1;
}

.cat-name-fb {
  font-size: 0.85rem;
  font-weight: 500;
}

.toggle-comments-fb {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 14px 16px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  margin-bottom: 20px;
  cursor: pointer;
  user-select: none;
}

.toggle-cb-fb {
  margin-top: 2px;
  width: 16px;
  height: 16px;
  accent-color: #e94560;
  flex-shrink: 0;
}

.toggle-info-fb {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-label-fb {
  font-size: 0.85rem;
  font-weight: 500;
}

.toggle-desc-fb {
  font-size: 0.78rem;
  color: #8888aa;
}

.cat-warning-fb {
  text-align: center;
  color: #f9a825;
  font-size: 0.82rem;
  margin-bottom: 16px;
}

/* ---- Responsive ---- */
@media (max-width: 560px) {
  .step-panel {
    padding: 36px 24px;
    border-radius: 12px;
  }
  .title {
    font-size: 1.4rem;
  }
  .id-value {
    font-size: 1.2rem;
  }
  .recovery-box {
    flex-direction: column;
    align-items: stretch;
  }
  .copy-btn {
    align-self: flex-end;
  }
  .category-grid-fb {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
