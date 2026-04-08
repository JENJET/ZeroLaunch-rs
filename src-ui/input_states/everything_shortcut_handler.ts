import type { Ref } from 'vue'
import type { EverythingShortcutConfig, ShortcutConfig } from '../api/remote_config_types'
import type { ShortcutHandler } from './shortcut_handler'
import { matchShortcut } from './shortcut_handler'
import { invoke } from '@tauri-apps/api/core'

/**
 * Everything 面板实例接口
 */
export interface EverythingPanelInstance {
    moveSelection: (direction: number) => void
    launchSelected: () => void
    getSelectedPath: () => string | null
    togglePathMatch: () => boolean
}

/**
 * 右键菜单实例接口
 */
export interface SubMenuInstance {
    isVisible: () => boolean
    hideMenu: () => void
}

/**
 * 创建 Everything 页面的快捷键处理器
 * @param everythingShortcutConfig Everything 特有的快捷键配置
 * @param shortcutConfig 全局共用的快捷键配置（导航等）
 * @param panelRef Everything 面板引用
 * @param searchText Everything 搜索栏内容的引用
 * @param resultItemMenuRef 右键菜单引用
 * @returns 快捷键处理器实例
 */
export function createEverythingShortcutHandler(
    everythingShortcutConfig: Ref<EverythingShortcutConfig>,
    shortcutConfig: Ref<ShortcutConfig>,
    panelRef: Ref<EverythingPanelInstance | null>,
    searchText?: Ref<string>,
    resultItemMenuRef?: Ref<SubMenuInstance | null>,
): ShortcutHandler {
    return {
        handleKeyDown(event: KeyboardEvent): boolean {
            // Everything 特有的快捷键：在资源管理器中打开
            console.log(everythingShortcutConfig.value)
            if (matchShortcut(event, everythingShortcutConfig.value.enable_path_match)) {
                event.preventDefault()
                const newState = panelRef.value?.togglePathMatch?.() ?? true
                invoke('everything_enable_path_match', { enable: newState })
                return true
            }

            // 导航快捷键：向下移动
            if (event.key === 'ArrowDown' || matchShortcut(event, shortcutConfig.value.arrow_down)) {
                event.preventDefault()
                panelRef.value?.moveSelection(1)
                return true
            }

            // 导航快捷键：向上移动
            if (event.key === 'ArrowUp' || matchShortcut(event, shortcutConfig.value.arrow_up)) {
                event.preventDefault()
                panelRef.value?.moveSelection(-1)
                return true
            }

            // 确认选择
            if (event.key === 'Enter') {
                event.preventDefault()
                panelRef.value?.launchSelected()
                return true
            }

            // ESC 键：清空搜索栏或退出 Everything 模式
            if (event.key === 'Escape') {
                event.preventDefault()
                // ✅ ESC 键处理优先级：
                // 1. 有右键菜单 → 关闭菜单
                // 2. 没有菜单但有搜索文本 → 清空搜索文本
                // 3. 没有菜单且没有搜索文本 → 退出 Everything 模式（返回主搜索）
                
                const isMenuVisible = resultItemMenuRef?.value?.isVisible() ?? false
                
                if (isMenuVisible) {
                    // 优先级 1：关闭右键菜单
                    resultItemMenuRef?.value?.hideMenu()
                } else if (searchText && searchText.value.length > 0) {
                    // 优先级 2：清空搜索文本
                    searchText.value = ''
                } else {
                    // 优先级 3：退出 Everything 模式（返回主搜索）
                    return false
                }
                return true
            }

            // Alt 键：在 Everything 模式中忽略（不切换到最近程序列表）
            if (event.key === 'Alt') {
                event.preventDefault()
                return true
            }

            // 未处理的事件
            return false
        },
    }
}
