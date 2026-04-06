import { ref, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ProgramDisplayInfo } from '../api/program'

export function useProgramSearch() {
    const searchKeyword = ref('')
    const loading = ref(false)
    const programList = ref<ProgramDisplayInfo[]>([])
    const iconUrls = ref(new Map<string, string>())
    const iconLoading = ref(new Set<string>()) // 跟踪正在加载的图标
    const showAllMode = ref(false)
    let searchTimeout: number | undefined

    const loadIcon = async (row: ProgramDisplayInfo) => {
        const key = row.icon_request_json
        
        // 如果已经有缓存，直接返回
        if (iconUrls.value.has(key)) return
        
        // 如果已经在加载中，避免重复请求
        if (iconLoading.value.has(key)) return
        
        // 标记为加载中
        iconLoading.value.add(key)
        
        try {
            const data = await invoke<number[]>('load_program_icon', { programGuid: row.program_guid })

            // Use Blob to optimize performance and avoid base64 conversion overhead
            const bytes = new Uint8Array(data)
            const blob = new Blob([bytes], { type: 'image/png' })
            const url = URL.createObjectURL(blob)

            iconUrls.value.set(key, url)
        } catch (e) {
            console.error('Failed to load icon', e)
        } finally {
            // 移除加载状态
            iconLoading.value.delete(key)
        }
    }

    const handleSearch = () => {
        if (searchTimeout) clearTimeout(searchTimeout)
        searchTimeout = window.setTimeout(async () => {
            loading.value = true
            try {
                const results = await invoke<ProgramDisplayInfo[]>('command_search_programs_lightweight', {
                    keyword: searchKeyword.value,
                    loadAll: showAllMode.value
                })
                programList.value = results
                // Load icons for results
                results.forEach(loadIcon)
            } catch (e) {
                console.error('Search failed', e)
            } finally {
                loading.value = false
            }
        }, 300)
    }

    const toggleShowAll = () => {
        showAllMode.value = !showAllMode.value
        handleSearch()
    }

    const getIconUrl = (icon_request_json: string) => {
        return iconUrls.value.get(icon_request_json) || ''
    }
    
    const isIconLoading = (icon_request_json: string) => {
        return iconLoading.value.has(icon_request_json)
    }

    const refreshIcon = async (program: ProgramDisplayInfo) => {
        const key = program.icon_request_json
        const oldUrl = iconUrls.value.get(key)
        if (oldUrl) {
            URL.revokeObjectURL(oldUrl)
            iconUrls.value.delete(key)
        }
        // 重新加载图标
        await loadIcon(program)
    }

    // Clean up resources
    onUnmounted(() => {
        if (searchTimeout) clearTimeout(searchTimeout)
        iconUrls.value.forEach(url => URL.revokeObjectURL(url))
        iconUrls.value.clear()
        iconLoading.value.clear()
    })

    return {
        searchKeyword,
        loading,
        programList,
        showAllMode,
        handleSearch,
        toggleShowAll,
        getIconUrl,
        isIconLoading,
        refreshIcon
    }
}
