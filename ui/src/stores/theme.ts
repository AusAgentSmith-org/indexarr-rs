import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  const saved = localStorage.getItem('indexarr-theme') || 'light'
  document.documentElement.setAttribute('data-theme', saved)

  const theme = ref<'light' | 'dark'>(saved as 'light' | 'dark')

  function toggle() {
    theme.value = theme.value === 'light' ? 'dark' : 'light'
    document.documentElement.setAttribute('data-theme', theme.value)
    localStorage.setItem('indexarr-theme', theme.value)
  }

  return { theme, toggle }
})
