<script setup lang="ts">
import { useI18n } from '../../composables/useI18n'
import { useTheme } from '../../composables/useTheme'

const { t, locale, setLocale } = useI18n()
const { theme, setTheme } = useTheme()
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('language') }}</label>
    <div class="segmented-options">
      <button
        class="segmented-btn"
        :class="{ active: locale === 'zh' }"
        @click="setLocale('zh')"
      >
        {{ t('chinese') }}
      </button>
      <button
        class="segmented-btn"
        :class="{ active: locale === 'en' }"
        @click="setLocale('en')"
      >
        {{ t('english') }}
      </button>
    </div>
  </div>

  <div class="setting-group">
    <label class="setting-label">{{ t('theme') }}</label>
    <div class="segmented-options">
      <button
        class="segmented-btn"
        :class="{ active: theme === 'dark' }"
        @click="setTheme('dark')"
      >
        {{ t('darkMode') }}
      </button>
      <button
        class="segmented-btn"
        :class="{ active: theme === 'light' }"
        @click="setTheme('light')"
      >
        {{ t('lightMode') }}
      </button>
    </div>
  </div>
</template>
