<template>
  <div
    v-if="uiConfig.footer_height > 0"
    class="footer"
    :style="{ 
      height: uiConfig.footer_height + 'px',
      backgroundColor: uiConfig.search_bar_background_color, 
      fontSize: Math.round(uiConfig.footer_height * uiConfig.footer_font_size * layoutConstants.fontSizeRatio) + 'px', 
      fontFamily: uiConfig.footer_font_family, 
    }"
    @mousedown="startDrag"
  >
    <div class="footer-left">
      <span
        class="status-text"
        :style="{ color: uiConfig.footer_font_color, fontFamily: uiConfig.footer_font_family }"
      >{{
        leftText || displayTips }}</span>
    </div>
    <div class="footer-center" />
    <div class="footer-right">
      <span
        class="open-text"
        :style="{ color: uiConfig.footer_font_color, fontFamily: uiConfig.footer_font_family }"
      >
        {{ statusText }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ResolvedUIConfig, AppConfig } from '../../api/remote_config_types'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed } from 'vue'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { FALLBACK_DEFAULTS } from '../../api/remote_config_types'

const layoutConstants = {
  fontSizeRatio: 0.01,
}

const props = defineProps<{
  uiConfig: ResolvedUIConfig;
  appConfig: AppConfig;
  statusText: string;
  leftText?: string;
}>()

const configStore = useRemoteConfigStore()
const { appVersion, defaultAppConfig } = storeToRefs(configStore)

// 获取默认提示文字（优先从 store 获取，fallback 到常量）
const defaultTips = computed(() => {
  return defaultAppConfig.value?.tips 
    || (appVersion.value ? `${FALLBACK_DEFAULTS.tips_base} v${appVersion.value}` : FALLBACK_DEFAULTS.tips_base)
})

// 实际显示的提示文字：优先使用配置的 tips，如果为空则使用默认值
const displayTips = computed(() => {
  return props.appConfig.tips || defaultTips.value
})

const startDrag = (e: MouseEvent) => {
  if (!props.appConfig.is_enable_drag_window) return
  if (e.button !== 0) return
  getCurrentWindow().startDragging()
}
</script>

<style scoped>
.footer {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  width: 100%;
  flex-shrink: 0;
}

.footer-left {
  margin-left: 16px;
  flex-shrink: 0;
}

.footer-right {
  margin-right: 16px;
  flex-shrink: 0;
}

.footer-center {
  flex-grow: 1;
}

.status-text,
.open-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
