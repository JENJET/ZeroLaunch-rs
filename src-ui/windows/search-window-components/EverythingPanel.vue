<template>
  <div
    class="everything-panel"
    :style="{
      position: 'relative',
      flex: 1,
      minHeight: 0,
    }"
  >
    <div
      v-if="results.length === 0 && !isSearching"
      class="no-results"
      :style="{
        color: uiConfig.item_font_color,
        fontFamily: uiConfig.result_item_font_family,
        fontSize: Math.round(uiConfig.result_item_height * 0.4) + 'px'
      }"
    >
      {{ props.searchText ? (currentArch === 'aarch64' ? t('everything.not_supported') : t('everything.no_results')) : t('everything.no_results') }}
    </div>
    <div
      v-if="message"
      class="message-overlay"
      :style="{
        color: uiConfig.item_font_color,
        fontFamily: uiConfig.result_item_font_family,
        fontSize: Math.round(uiConfig.result_item_height * 0.4) + 'px',
        backgroundColor: hoverColor,
      }"
    >
      {{ message }}
    </div>
    <div
      v-else
      ref="resultsListRef"
      class="results-list"
      @scroll="handleScroll"
    >
      <!-- 虚拟滚动：只渲染可见区域的项目 -->
      <div
        v-for="(item, index) in visibleItems"
        :key="item[0]"
        class="result-item"
        :class="{ 'selected': selectedIndex === getActualIndex(index) }"
        :style="{
          '--hover-color': hoverColor,
          '--selected-color': uiConfig.selected_item_color,
          height: uiConfig.result_item_height + 'px',
          transform: `translateY(${getActualIndex(index) * uiConfig.result_item_height}px)`,
        }"
        @click="handleItemClick(getActualIndex(index))"
        @contextmenu.prevent="handleItemContextmenu(getActualIndex(index), $event)"
      >
        <div
          class="icon"
          :style="{
            width: Math.round(uiConfig.result_item_height * layoutConstants.iconSizeRatio) + 'px',
            height: Math.round(uiConfig.result_item_height * layoutConstants.iconSizeRatio) + 'px',
            marginLeft: Math.round(uiConfig.result_item_height * layoutConstants.iconMarginRatio) + 'px',
            marginRight: Math.round(uiConfig.result_item_height * layoutConstants.iconMarginRatio) + 'px',
          }"
        >
          <img
            :src="iconMap.get(item[1]) || '/tauri.svg'"
            class="custom-image"
            alt="icon"
            @load="onIconLoad(item[1])"
            @error="onIconError(item[1])"
          >
        </div>
        <div class="item-content">
          <div
            class="file-name"
            :style="{
              fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size * layoutConstants.fontSizeRatio) + 'px',
              fontFamily: uiConfig.result_item_font_family,
              color: uiConfig.item_font_color,
              fontWeight: '600',
            }"
          >
            {{ getFileName(item[1]) }}
          </div>
          <div
            class="file-path"
            :style="{
              fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size * layoutConstants.fontSizeRatio * 0.65) + 'px',
              fontFamily: uiConfig.result_item_font_family,
              color: getPathColor(),
            }"
          >
            {{ getDirectoryPath(item[1]) }}
          </div>
        </div>
      </div>
      <!-- 占位元素，保持滚动条正确 -->
      <div
        class="spacer"
        :style="{
          height: results.length * uiConfig.result_item_height + 'px',
        }"
      ></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { getColorWithReducedOpacity } from '../../utils/color'
import type { ResolvedUIConfig, AppConfig } from '../../api/remote_config_types'

const props = defineProps<{
    searchText: string;
    uiConfig: ResolvedUIConfig;
    appConfig: AppConfig;
    hoverColor: string;
}>()

const emit = defineEmits<{
    (e: 'item-click', index: number): void;
    (e: 'item-contextmenu', index: number, event: MouseEvent): void;
}>()

const { t } = useI18n()

const layoutConstants = {
    iconSizeRatio: 0.6,
    iconMarginRatio: 0.2,
    fontSizeRatio: 0.01,
}

const results = ref<Array<[number, string, string]>>([])
const selectedIndex = ref(0)
const isSearching = ref(false)
const pendingSearchText = ref<string | null>(null)
const resultsListRef = ref<HTMLElement | null>(null)
const iconMap = ref<Map<string, string>>(new Map())
const enablePathMatch = ref<boolean>(false)
const currentArch = ref<string>('')
const message = ref<string | null>(null)
let messageTimeout: number | null = null

// 虚拟滚动相关
const scrollTop = ref(0)
const visibleCount = computed(() => props.appConfig.search_result_count)
const bufferSize = 5 // 缓冲区大小（上下各多渲染几个）

// 计算可见的项目范围
const visibleRange = computed(() => {
    const start = Math.max(0, Math.floor(scrollTop.value / props.uiConfig.result_item_height) - bufferSize)
    const end = Math.min(
        results.value.length,
        start + visibleCount.value + bufferSize * 2
    )
    return { start, end }
})

// 可见的项目
const visibleItems = computed(() => {
    const { start, end } = visibleRange.value
    return results.value.slice(start, end)
})

// 获取实际索引（考虑偏移）
const getActualIndex = (visibleIndex: number) => {
    return visibleRange.value.start + visibleIndex
}

// 正在加载的图标请求（用于取消）
const loadingIcons = ref<Map<string, AbortController>>(new Map())

const showMessage = (msg: string) => {
    message.value = msg
    if (messageTimeout) clearTimeout(messageTimeout)
    messageTimeout = window.setTimeout(() => {
        message.value = null
    }, 2000)
}

// 从完整路径中提取文件名（包含扩展名）
const getFileName = (fullPath: string): string => {
    const parts = fullPath.split('\\')
    return parts[parts.length - 1] || fullPath
}

// 从完整路径中提取目录路径
const getDirectoryPath = (fullPath: string): string => {
    const parts = fullPath.split('\\')
    if (parts.length <= 1) return fullPath
    return parts.slice(0, -1).join('\\')
}

// 计算路径颜色（使用统一的 color.ts 函数）
const getPathColor = (): string => {
    return getColorWithReducedOpacity(props.uiConfig.item_font_color, 0.6)
}

// 处理滚动事件
const handleScroll = () => {
    if (resultsListRef.value) {
        scrollTop.value = resultsListRef.value.scrollTop
    }
}

// 加载单个图标（带取消支持）
const loadSingleIcon = async (path: string) => {
    // 如果已经有正在加载的请求，先取消
    const existingController = loadingIcons.value.get(path)
    if (existingController) {
        existingController.abort()
    }
    
    // 创建新的 AbortController
    const controller = new AbortController()
    loadingIcons.value.set(path, controller)
    
    try {
        const iconData = await invoke<number[]>('get_everything_icon', { path })
        
        // 检查是否被取消
        if (controller.signal.aborted) {
            return
        }
        
        if (iconData && iconData.length > 0) {
            const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' })
            const url = URL.createObjectURL(blob)
            iconMap.value.set(path, url)
        }
    } catch (e) {
        if (e.name !== 'AbortError') {
            console.error('Failed to load icon for', path, e)
        }
    } finally {
        loadingIcons.value.delete(path)
    }
}

// 加载可见区域的图标
const loadVisibleIcons = async () => {
    const { start, end } = visibleRange.value
    const pathsToLoad = results.value.slice(start, end).map(item => item[1])
    
    // 并行加载可见区域的图标
    await Promise.all(pathsToLoad.map(path => loadSingleIcon(path)))
}

// 图标加载成功
const onIconLoad = (path: string) => {
    // 可以在这里添加日志或统计
}

// 图标加载失败
const onIconError = (path: string) => {
    console.warn('Icon load failed for:', path)
}

// 取消所有正在加载的图标请求
const cancelAllIconRequests = () => {
    loadingIcons.value.forEach(controller => controller.abort())
    loadingIcons.value.clear()
}

onUnmounted(() => {
    // 清理所有图标 URL
    iconMap.value.forEach(url => URL.revokeObjectURL(url))
    // 取消所有进行中的请求
    cancelAllIconRequests()
})

const performSearch = async (text: string) => {
    if (isSearching.value) {
        pendingSearchText.value = text
        return
    }

    isSearching.value = true
    
    // 取消之前的图标请求
    cancelAllIconRequests()
    iconMap.value.forEach(url => URL.revokeObjectURL(url))
    iconMap.value.clear()
    
    try {
        const searchResults: Array<[number, string, string]> = await invoke('handle_everything_search', { searchText: text })
        results.value = searchResults
        selectedIndex.value = 0
        scrollTop.value = 0
        if (resultsListRef.value) {
            resultsListRef.value.scrollTop = 0
        }
        // 只加载可见区域的图标
        await loadVisibleIcons()
    } catch (error) {
        console.error('Everything search failed:', error)
    } finally {
        isSearching.value = false
        if (pendingSearchText.value !== null) {
            const nextText = pendingSearchText.value
            pendingSearchText.value = null
            if (nextText !== text) {
                performSearch(nextText)
            }
        }
    }
}

watch(() => props.searchText, (newText) => {
    performSearch(newText)
})

// 监听滚动，动态加载图标
watch(visibleRange, () => {
    loadVisibleIcons()
}, { deep: true })

const handleItemClick = (index: number) => {
    selectedIndex.value = index
    emit('item-click', index)
    launchItem(index)
}

const handleItemContextmenu = (index: number, event: MouseEvent) => {
    selectedIndex.value = index
    emit('item-contextmenu', index, event)
}

const launchItem = async (index: number) => {
    const item = results.value[index]
    if (item) {
        try {
            await invoke('launch_everything_item', { path: item[1] })
        } catch (error) {
            console.error('Failed to launch everything item:', error)
        }
    }
}

const moveSelection = (direction: number) => {
    if (results.value.length === 0) return
    
    let newIndex = selectedIndex.value + direction
    if (newIndex < 0) {
        newIndex = results.value.length - 1
    } else if (newIndex >= results.value.length) {
        newIndex = 0
    }
    
    selectedIndex.value = newIndex
    scrollToSelected()
}

const scrollToSelected = () => {
    if (!resultsListRef.value) return
    const selectedEl = resultsListRef.value.children[selectedIndex.value] as HTMLElement
    if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest' })
    }
}

defineExpose({
    moveSelection,
    launchSelected: () => launchItem(selectedIndex.value),
    resultsCount: () => results.value.length,
    getSelectedPath: () => {
        const item = results.value[selectedIndex.value]
        return item ? item[1] : null
    },
    enablePathMatch,
    togglePathMatch: () => {
        enablePathMatch.value = !enablePathMatch.value
        showMessage(enablePathMatch.value ? t('everything.path_match_enabled') : t('everything.path_match_disabled'))
        return enablePathMatch.value
    },
})

onMounted(async () => {
    currentArch.value = await invoke('command_get_arch')
    if (props.searchText) {
        performSearch(props.searchText)
    }
})
</script>

<style scoped>
.everything-panel {
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
}

.everything-panel::-webkit-scrollbar {
    width: 6px;
}

.everything-panel::-webkit-scrollbar-track {
    background: transparent;
}

.everything-panel::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
}

.everything-panel::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.4);
}

.no-results {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    opacity: 0.6;
}

.result-item {
    display: flex;
    align-items: center;
    cursor: pointer;
    transition: background-color 0.2s;
    flex-shrink: 0;
    position: absolute;
    width: 100%;
    left: 0;
}

.result-item:hover {
    background-color: var(--hover-color);
}

.result-item.selected {
    background-color: var(--selected-color);
}

.icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.custom-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 6px;
}

.item-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    overflow: hidden;
    flex: 1;
    padding-right: 12px;
    height: 100%;
}

.file-name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    line-height: 1.2;
}

.file-path {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    opacity: 0.8;
    line-height: 1.2;
}

.message-overlay {
    position: absolute;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    border-radius: 4px;
    z-index: 10;
    pointer-events: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.spacer {
    position: relative;
    width: 100%;
    pointer-events: none;
}
</style>
