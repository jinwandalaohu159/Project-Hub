import { ref } from 'vue'
import en from '../i18n/en'
import zh from '../i18n/zh'

export type Locale = 'en' | 'zh'
export type TranslationKey = keyof typeof en

const LOCALE_KEY = 'project-hub-locale'

/** The language chosen last time (it also reaches the update helper's window); Chinese at first. */
function savedLocale(): Locale {
  try {
    return window.localStorage.getItem(LOCALE_KEY) === 'en' ? 'en' : 'zh'
  } catch {
    return 'zh'
  }
}

const locale = ref<Locale>(savedLocale())

const messages: Record<Locale, Record<TranslationKey, string>> = { en, zh }

export function useI18n() {
  function t(key: TranslationKey, params?: Record<string, string | number>): string {
    let text = messages[locale.value][key] ?? key
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        text = text.replace(`{${k}}`, String(v))
      }
    }
    return text
  }

  function setLocale(l: Locale) {
    locale.value = l
    try {
      window.localStorage.setItem(LOCALE_KEY, l)
    } catch {
      // Unavailable storage only means the choice is not remembered.
    }
  }

  return { locale, t, setLocale }
}