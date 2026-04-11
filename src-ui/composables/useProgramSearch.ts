import { ref, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ProgramDisplayInfo } from '../api/program'

export interface UseProgramSearchOptions {
    getAliasFn?: (path: string) => string[]
}

export function useProgramSearch(options?: UseProgramSearchOptions) {
    const searchKeyword = ref('')
    const loading = ref(false)
    const programList = ref<ProgramDisplayInfo[]>([])
    const iconUrls = ref(new Map<string, string>())
    const iconLoading = ref(new Set<string>()) // 跟踪正在加载的图标
    const showAllMode = ref(false)
    const allPrograms = ref<ProgramDisplayInfo[]>([]) // 存储全量数据用于前端过滤
    const isLoaded = ref(false) // 标记是否已加载过全量数据
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

    // 前端搜索过滤函数
    const filterPrograms = (keyword: string, programs: ProgramDisplayInfo[]): ProgramDisplayInfo[] => {
        if (!keyword.trim()) return programs
        const lowerKeyword = keyword.toLowerCase()
        return programs.filter(p => {
            const nameMatch = p.name.toLowerCase().includes(lowerKeyword)
            const pathMatch = p.path.toLowerCase().includes(lowerKeyword)
            // 如果提供了别名搜索函数，也搜索别名
            const aliasMatch = options?.getAliasFn
                ? (options.getAliasFn(p.path) || []).some(a => a.toLowerCase().includes(lowerKeyword))
                : false
            return nameMatch || pathMatch || aliasMatch
        })
    }

    // 计算属性：显示的程序列表（支持前端过滤）
    const filteredProgramList = computed(() => {
        if (!showAllMode.value || !allPrograms.value.length) {
            return programList.value
        }
        return filterPrograms(searchKeyword.value, allPrograms.value)
    })

    const handleSearch = () => {
        // 在"显示全部"模式下，如果已加载过全量数据，直接使用前端过滤
        if (showAllMode.value && isLoaded.value) {
            return // 无需调用后端，直接使用缓存数据
        }
        
        if (searchTimeout) clearTimeout(searchTimeout)
        searchTimeout = window.setTimeout(async () => {
            loading.value = true
            try {
                const results = await invoke<ProgramDisplayInfo[]>('command_search_programs_lightweight', {
                    keyword: showAllMode.value ? '' : searchKeyword.value, // 显示全部时不传关键词给后端
                    loadAll: showAllMode.value
                })
                programList.value = results
                
                // 在"显示全部"模式下，缓存全量数据到本地
                if (showAllMode.value) {
                    allPrograms.value = results
                    isLoaded.value = true // 标记已加载
                    results.forEach(loadIcon)
                } else {
                    allPrograms.value = []
                    isLoaded.value = false // 普通模式不标记
                    results.forEach(loadIcon)
                }
            } catch (e) {
                console.error('Search failed', e)
            } finally {
                loading.value = false
            }
        }, 300)
    }

    const toggleShowAll = () => {
        showAllMode.value = !showAllMode.value
        if (showAllMode.value) {
            // 切换到"显示全部"时，如果还未加载过，则加载
            if (!isLoaded.value) {
                searchKeyword.value = ''
                handleSearch()
            }
        } else {
            // 切换回"普通模式"时，清空状态
            searchKeyword.value = ''
            isLoaded.value = false
            allPrograms.value = []
            handleSearch()
        }
    }

    // 重置加载状态（用于页面关闭时调用）
    const resetLoadedState = () => {
        isLoaded.value = false
        allPrograms.value = []
        programList.value = []
        searchKeyword.value = ''
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
        filteredProgramList, // 新增：支持前端过滤的列表
        showAllMode,
        handleSearch,
        toggleShowAll,
        resetLoadedState, // 新增：重置加载状态
        getIconUrl,
        isIconLoading,
        refreshIcon
    }
}
