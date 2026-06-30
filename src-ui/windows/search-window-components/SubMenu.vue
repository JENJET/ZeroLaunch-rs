<template>
  <div
    v-if="visible"
    ref="submenuRef"
    class="submenu"
    :style="submenuStyle"
    @contextmenu.prevent
  >
    <div
      v-for="(item, index) in menuItems"
      :key="index"
      class="submenu-item"
      :class="{ 'selected': selectedIndex === index }"
      :style="itemStyle"
      @click="handleItemClick(index)"
    >
      <div
        class="submenu-icon"
        :style="iconStyle"
      >
        <component :is="item.icon" />
      </div>
      <div
        class="submenu-item-name"
        :style="nameStyle"
      >
        {{ item.name }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { Component } from 'vue'

export interface MenuItem {
    name: string;
    icon: Component;
    action: Function;
}

export interface Position {
    top: number;
    left: number;
}

export interface WindowSize {
    width: number;
    height: number;
}

const props = defineProps<{
    itemHeight: number;
    windowSize: WindowSize;
    menuItems: MenuItem[];
    isDark?: boolean;
    cornerRadius?: number;
    hoverColor?: string;
    selectedColor?: string;
    itemFontColor?: string;
    itemFontSizePercent?: number;
}>()

// 组件状态
const visible = ref(false)
const selectedIndex = ref(0)
const submenuRef = ref<HTMLElement | null>(null)
const position = ref<Position>({ top: 0, left: 0 })
const anchorPosition = ref<Position>({ top: 0, left: 0 })

// 计算样式
const submenuStyle = computed(() => {
    return {
        top: `${position.value.top}px`,
        left: `${position.value.left}px`,
        border: `1px solid ${props.isDark ? '#3d3d3d' : '#bdbdbd'}`,
        borderRadius: `${(props.cornerRadius || 8) / 2}px`,
        backgroundColor: props.isDark ? '#252525' : '#ffffff',
        boxShadow: '0 2px 12px 0 rgba(0, 0, 0, 0.1)',
        zIndex: 1000,
        position: 'absolute' as const, // Ensure it's positioned absolutely
    }
})

const itemStyle = computed(() => {
    return {
        '--hover-color': props.hoverColor || '#f5f5f5',
        '--selected-color': props.selectedColor || '#e6f7ff',
        height: `${props.itemHeight * 0.6}px`,
        display: 'flex',
        alignItems: 'center',
        cursor: 'pointer',
        padding: '0 8px',
        transition: 'background-color 0.2s',
    }
})

const iconStyle = computed(() => {
    return {
        width: `${props.itemHeight * 0.4}px`,
        height: `${props.itemHeight * 0.4}px`,
        marginLeft: `${props.itemHeight * 0.1}px`,
        marginRight: `${props.itemHeight * 0.1}px`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    }
})

const nameStyle = computed(() => {
    const fontSize = Math.min(props.itemHeight * (props.itemFontSizePercent || 14) / 100, props.itemHeight * 0.4) * 0.75
    return {
        fontSize: `${fontSize}px`,
        color: props.itemFontColor || (props.isDark ? '#e0e0e0' : '#333333'),
        marginRight: `${props.itemHeight * 0.1}px`,
        whiteSpace: 'nowrap',
    }
})

// 计算菜单尺寸
const calculateMenuSize = () => {
    if (!submenuRef.value) {
        return {
            width: 0,
            height: props.menuItems.length * props.itemHeight * 0.6 + 8,
        }
    }

    return {
        width: submenuRef.value.offsetWidth,
        height: submenuRef.value.offsetHeight,
    }
}

const calculateMenuBounds = () => {
    const container = submenuRef.value?.parentElement
    if (container) {
        return {
            width: container.clientWidth,
            height: container.clientHeight,
        }
    }

    return props.windowSize
}

// 计算调整后的位置，确保菜单完全在窗口内
const calculateAdjustedPosition = (anchorTop: number, anchorLeft: number) => {
    const menuSize = calculateMenuSize()
    const { width, height } = calculateMenuBounds()
    let top = anchorTop
    let left = anchorLeft

    if (top + menuSize.height > height) {
        top = anchorTop - menuSize.height
    }
    
    // 确保菜单不会超出右边界
    if (left + menuSize.width > width) {
        left = width - menuSize.width - 5 // 5px 边距
    }

    // 确保菜单不会超出上边界
    if (top < 0) {
        top = 5 // 5px 边距
    }

    // 极端情况下菜单比窗口高，优先保证顶部可见
    if (top + menuSize.height > height) {
        top = Math.max(5, height - menuSize.height - 5)
    }

    // 确保菜单不会超出左边界
    if (left < 0) {
        left = 5 // 5px 边距
    }
    
    return { top, left }
}

// 对外暴露的方法
const selectNext = () => {
    selectedIndex.value = (selectedIndex.value + 1) % props.menuItems.length
}

const selectPrevious = () => {
    selectedIndex.value = (selectedIndex.value - 1 + props.menuItems.length) % props.menuItems.length
}

watch(visible, async (newValue) => {
    if (newValue) {
        await nextTick()
        // 菜单显示后重新计算位置
        const adjustedPosition = calculateAdjustedPosition(anchorPosition.value.top, anchorPosition.value.left)
        if (submenuRef.value) {
            position.value.top = adjustedPosition.top
            position.value.left = adjustedPosition.left
        }
    }
})

// 修改后的showMenu方法，需要传入位置参数
const showMenu = async (newPosition: Position) => {
    initMenu()
    anchorPosition.value = newPosition
    position.value = newPosition
    visible.value = true
    await nextTick()
    const adjustedPosition = calculateAdjustedPosition(anchorPosition.value.top, anchorPosition.value.left)
    position.value.top = adjustedPosition.top
    position.value.left = adjustedPosition.left
}

const hideMenu = () => {
    visible.value = false
}

const selectCurrent = () => {
    if (visible.value && props.menuItems[selectedIndex.value]) {
        props.menuItems[selectedIndex.value].action()
        hideMenu()
    }
}

const initMenu = () => {
    selectedIndex.value = 0
}

// 处理菜单项点击
const handleItemClick = (index: number) => {
    selectedIndex.value = index
    selectCurrent()
}

const isVisible = () => {
    return visible.value
}

defineExpose({
    showMenu,
    hideMenu,
    selectNext,
    selectPrevious,
    selectCurrent,
    isVisible,
})
</script>

<style scoped>
.submenu {
    position: absolute;
    display: flex;
    flex-direction: column;
    padding: 4px 0;
    overflow: hidden;
}

.submenu-item {
    width: 100%;
    box-sizing: border-box;
}

.submenu-item:hover,
.submenu-item.selected {
    background-color: var(--selected-color);
}
</style>
