import { ref } from 'vue'

export type AppTheme = 'dark' | 'light'

const storageKey = 'project-hub-theme'
const theme = ref<AppTheme>('dark')
let initialized = false

function applyTheme(value: AppTheme) {
  if (typeof document === 'undefined') return
  document.documentElement.dataset.theme = value
  document.documentElement.style.colorScheme = value
}

function readSavedTheme(): AppTheme {
  if (typeof window === 'undefined') return 'dark'
  return window.localStorage.getItem(storageKey) === 'light' ? 'light' : 'dark'
}

export function useTheme() {
  function initTheme() {
    if (initialized) return
    initialized = true
    theme.value = readSavedTheme()
    applyTheme(theme.value)
  }

  function setTheme(value: AppTheme) {
    theme.value = value
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(storageKey, value)
    }
    applyTheme(value)
  }

  initTheme()

  return { theme, setTheme }
}
