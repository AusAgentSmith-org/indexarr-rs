import { ref, onMounted, onUnmounted } from 'vue'

export function useAutoRefresh(callback: () => Promise<void>, intervalMs: number) {
  const isRefreshing = ref(false)
  let timer: ReturnType<typeof setInterval> | null = null

  async function refresh() {
    if (isRefreshing.value) return
    isRefreshing.value = true
    try {
      await callback()
    } catch (err) {
      console.warn('[auto-refresh] callback error:', err)
    } finally {
      isRefreshing.value = false
    }
  }

  function start() {
    if (timer) return
    timer = setInterval(refresh, intervalMs)
  }

  function stop() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  onMounted(() => {
    refresh()
    start()
  })

  onUnmounted(() => {
    stop()
  })

  return { isRefreshing, refresh, start, stop }
}
