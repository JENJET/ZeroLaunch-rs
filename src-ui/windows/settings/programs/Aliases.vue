<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('program_index.setting_alias') }}
    </h2>
    <div class="content-container">
      <div class="search-bar-row">
        <el-input
          v-model="searchKeyword"
          :placeholder="t('icon_management.search_placeholder')"
          :prefix-icon="Search"
          clearable
          @input="handleSearch"
          class="search-input"
        />
      </div>

      <div class="table-wrapper">
        <el-table
          v-loading="loading"
          :data="sortedProgramList"
          :style="{ width: '100%' }"
          height="100%"
        >
          <el-table-column
            :label="t('icon_management.icon')"
            width="60"
          >
            <template #default="scope">
              <div class="icon-container">
                <!-- 加载中显示spinner -->
                <div v-if="isIconLoading(scope.row.icon_request_json) && !getIconUrl(scope.row.icon_request_json)" class="icon-spinner">
                  <div class="spinner"></div>
                </div>
                <!-- 有图标时显示图片 -->
                <img
                  v-else
                  :src="getIconUrl(scope.row.icon_request_json)"
                  class="program-icon"
                  alt="icon"
                >
              </div>
            </template>
          </el-table-column>

          <el-table-column
            :label="t('icon_management.program_name')"
            prop="name"
            width="200"
          />

          <el-table-column
            :label="t('icon_management.path')"
            prop="path"
            show-overflow-tooltip
          />

          <el-table-column :label="t('program_index.setting_alias')">
            <template #default="{ row }">
              <div class="alias-tags">
                <el-tag
                  v-for="(alias, index) in getAliases(row.path)"
                  :key="index"
                  size="small"
                  class="alias-tag"
                  closable
                  @close="removeAliasDirectly(row, index)"
                >
                  {{ alias }}
                </el-tag>
              </div>
            </template>
          </el-table-column>

          <el-table-column
            :label="t('program_index.operation')"
            width="120"
            fixed="right"
          >
            <template #default="{ row }">
              <el-button
                size="small"
                type="primary"
                @click="openEditDialog(row)"
              >
                {{ t('program_index.setting_alias') }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>

    <el-dialog
      v-if="editingProgram"
      v-model="dialogVisible"
      :title="t('settings.edit_program_alias', { name: editingProgram.name })"
      width="500"
      @close="closeDialog"
    >
      <div style="display: flex; flex-direction: column; gap: 10px;">
        <div
          v-for="(alias, index) in dialogAliases"
          :key="index"
          style="display: flex; align-items: center; gap: 10px;"
        >
          <el-input
            :model-value="alias"
            :placeholder="t('settings.enter_alias')"
            @update:model-value="(newValue: string) => updateAliasInDialog(index, newValue)"
          />
          <el-button
            type="danger"
            @click="removeAliasInDialog(index)"
          >
            {{ t('settings.delete') }}
          </el-button>
        </div>
      </div>
      <template #footer>
        <div class="dialog-footer">
          <el-button
            style="width: 100%; margin-bottom: 10px;"
            @click="addAliasInDialog"
          >
            {{
              t('settings.add_alias') }}
          </el-button>
          <el-button
            type="primary"
            @click="closeDialog"
          >
            {{ t('settings.close') }}
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { ElButton, ElTag, ElInput, ElTable, ElTableColumn, ElDialog } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import type { ProgramDisplayInfo } from '../../../api/program'
import { useProgramSearch } from '../../../composables/useProgramSearch'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

// 组件名称，用于 keep-alive 缓存
defineOptions({
    name: 'Aliases'
})

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const dialogVisible = ref(false)
const editingProgram = ref<ProgramDisplayInfo | null>(null)
const dialogAliases = ref<string[]>([]) // 临时编辑状态，关闭对话框时才保存

const program_alias = computed({
    get: () => config.value.program_manager_config.loader.program_alias,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { program_alias: value },
            },
        })
    },
})

const getAliases = (path: string) => {
    return program_alias.value[path] || []
}

const {
    searchKeyword,
    loading,
    filteredProgramList,
    showAllMode,
    handleSearch,
    toggleShowAll,
    resetLoadedState,
    getIconUrl,
    isIconLoading
} = useProgramSearch({
    getAliasFn: getAliases
})

// 监听窗口显示事件，用于在窗口重新显示时重置加载状态
let unlistenWindowShown: UnlistenFn | null = null

// 根据是否设置别名排序
const sortedProgramList = computed(() => {
    return [...filteredProgramList.value].sort((a, b) => {
        const aHasAlias = (program_alias.value[a.path] || []).length > 0
        const bHasAlias = (program_alias.value[b.path] || []).length > 0
        
        // 有别名排在前面
        if (aHasAlias && !bHasAlias) return -1
        if (!aHasAlias && bHasAlias) return 1
        return 0
    })
})

const removeAliasDirectly = async (row: ProgramDisplayInfo, index: number) => {
    const path = row.path
    const aliases = [...(program_alias.value[path] || [])]
    aliases.splice(index, 1)
    
    const newAliasMap = { ...program_alias.value }
    if (aliases.length === 0) {
        delete newAliasMap[path] // 没有别名时删除键
    } else {
        newAliasMap[path] = aliases
    }
    program_alias.value = newAliasMap
    
    // 同步到后端内存
    await configStore.updateRuntimeConfig({
        program_manager_config: {
            loader: { program_alias: newAliasMap }
        }
    })
}

const openEditDialog = (row: ProgramDisplayInfo) => {
    editingProgram.value = row
    // 复制当前别名到临时状态，不立即修改配置
    dialogAliases.value = [...(program_alias.value[row.path] || [])]
    dialogVisible.value = true
}

const updateAliasInDialog = (index: number, newValue: string) => {
    dialogAliases.value[index] = newValue
}

const removeAliasInDialog = (index: number) => {
    dialogAliases.value.splice(index, 1)
}

const addAliasInDialog = () => {
    dialogAliases.value.push('')
}

// 清理空别名并关闭对话框
const closeDialog = async () => {
    if (!editingProgram.value) {
        dialogVisible.value = false
        return
    }
    const path = editingProgram.value.path
    // 过滤掉空字符串别名
    const filteredAliases = dialogAliases.value.filter(a => a.trim() !== '')
    
    const newAliasMap = { ...program_alias.value }
    if (filteredAliases.length === 0) {
        delete newAliasMap[path] // 没有别名时删除键
    } else {
        newAliasMap[path] = filteredAliases
    }
    program_alias.value = newAliasMap
    dialogVisible.value = false
    
    // 只更新后端内存，不保存到文件
    await configStore.updateRuntimeConfig({
        program_manager_config: {
            loader: { program_alias: newAliasMap }
        }
    })
}

// 别名管理界面默认加载所有程序，以便正确排序和显示
onMounted(async () => {
    showAllMode.value = true
    handleSearch()
    
    // 监听窗口显示事件
    unlistenWindowShown = await listen('window-shown', () => {
        // 窗口重新显示时，重置加载状态并重新加载
        resetLoadedState()
        showAllMode.value = true
        handleSearch()
    })
})

// 页面关闭时清理事件监听
onUnmounted(() => {
    if (unlistenWindowShown) {
        unlistenWindowShown()
        unlistenWindowShown = null
    }
})
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.content-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
}

.search-bar-row {
    display: flex;
    gap: 10px;
    margin-bottom: 16px;
}

.search-input {
    flex: 1;
}

.table-wrapper {
    flex: 1;
    min-height: 0;
}

.program-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
}

.icon-container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
}

.icon-spinner {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}

.spinner {
    width: 60%;
    height: 60%;
    border: 2px solid rgba(128, 128, 128, 0.2);
    border-top-color: rgba(128, 128, 128, 0.8);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.alias-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
}

.alias-tag {
    margin-right: 4px;
    cursor: pointer;
}
</style>
