import { ref } from 'vue'
import en from '../i18n/en'
import zh from '../i18n/zh'

export type Locale = 'en' | 'zh'
export type TranslationKey = keyof typeof en

const locale = ref<Locale>('zh')

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
  }

  return { locale, t, setLocale }
}