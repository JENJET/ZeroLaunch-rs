<template>
    <div class="settings-page">
        <h2 class="page-title">
            {{ t('settings.icon_management') }}
        </h2>
        <div class="content-container">
            <el-form label-width="auto" class="settings-form">
                <el-form-item :label="t('icon_management.enable_online_icon_loading')">
                    <el-switch v-model="config.icon_manager_config.enable_online" @change="(val: boolean) =>
                        configStore.updateConfig({
                            icon_manager_config: { enable_online: val }
                        })
                    " />
                    <el-tooltip class="box-item" effect="dark" :content="t('icon_management.online_icon_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('icon_management.enable_icon_cache')">
                    <el-switch v-model="config.icon_manager_config.enable_icon_cache" @change="(val: boolean) =>
                        configStore.updateConfig({
                            icon_manager_config: { enable_icon_cache: val }
                        })
                    " />
                    <el-tooltip class="box-item" effect="dark" :content="t('icon_management.icon_cache_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item>
                    <el-button type="primary" @click="openIconCacheDir">
                        {{ t('icon_management.open_icon_cache_folder') }}
                    </el-button>
                </el-form-item>
            </el-form>

            <el-divider />

            <div class="custom-icon-section">
                <h3>
                    {{ config.icon_manager_config.enable_icon_cache ? t('icon_management.custom_program_icon') : t('icon_management.current_program_icon') }}
                </h3>

                <el-alert
                    :title="config.icon_manager_config.enable_icon_cache ? t('icon_management.icon_cache_enabled_note') : t('icon_management.icon_cache_disabled_note')"
                    type="info"
                    show-icon
                    :closable="false"
                    style="margin-bottom: 16px;" />

                <div class="table-container">
                    <div class="search-bar-row">
                        <el-input v-model="searchKeyword" :placeholder="t('icon_management.search_placeholder')"
                            prefix-icon="Search" clearable :disabled="showAllMode" @input="handleSearch" class="search-input" />
                        <el-button
                            :type="showAllMode ? 'primary' : 'default'"
                            @click="toggleShowAll"
                        >
                            {{ showAllMode ? t('icon_management.back_to_search') : t('icon_management.show_all') }}
                        </el-button>
                    </div>

                    <div class="table-wrapper">
                        <el-table v-loading="loading" :data="programList" style="width: 100%" height="100%">
                        <el-table-column :label="t('icon_management.icon')" width="80">
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
                                    />
                                </div>
                            </template>
                        </el-table-column>

                        <el-table-column :label="t('icon_management.program_name')" prop="name" width="200" />

                        <el-table-column :label="t('icon_management.path')" prop="path">
                            <template #default="scope">
                                <span class="path-text">{{ scope.row.path }}</span>
                            </template>
                        </el-table-column>

                        <el-table-column v-if="config.icon_manager_config.enable_icon_cache" :label="t('icon_management.actions')" width="180" fixed="right">
                            <template #default="scope">
                                <div class="action-buttons">
                                    <el-button size="small" type="primary" @click="handleChangeIcon(scope.row)">
                                        {{ t('icon_management.change_icon') }}
                                    </el-button>
                                    <el-button 
                                        v-if="config.icon_manager_config.enable_icon_cache" 
                                        size="small" 
                                        type="danger" 
                                        @click="handleResetCache(scope.row)"
                                    >
                                        {{ t('icon_management.reset_cache') }}
                                    </el-button>
                                </div>
                            </template>
                        </el-table-column>
                    </el-table>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { useI18n } from 'vue-i18n'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import type { ProgramDisplayInfo } from '../../api/program'
import { useProgramSearch } from '../../composables/useProgramSearch'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const {
    searchKeyword,
    loading,
    programList,
    showAllMode,
    handleSearch,
    toggleShowAll,
    getIconUrl,
    isIconLoading,
    refreshIcon
} = useProgramSearch()

const handleChangeIcon = async (program: ProgramDisplayInfo) => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Image or Executable',
                extensions: ['png', 'jpg', 'jpeg', 'ico', 'svg', 'exe', 'lnk']
            }]
        })

        if (selected && typeof selected === 'string') {
            await invoke('command_update_program_icon', {
                iconRequestJson: program.icon_request_json,
                newIconPath: selected
            })

            // Force refresh icon
            await refreshIcon(program)

            ElMessage.success(t('icon_management.update_success'))
        }
    } catch (e) {
        console.error('Failed to update icon', e)
        ElMessage.error(t('icon_management.update_failed') + `: ${e}`)
    }
}

const openIconCacheDir = async () => {
    try {
        await invoke('command_open_icon_cache_dir')
    } catch (e) {
        console.error('Failed to open icon cache dir', e)
    }
}

const handleResetCache = async (program: ProgramDisplayInfo) => {
    try {
        await invoke('command_reset_icon_cache', {
            iconRequestJson: program.icon_request_json
        })

        // Force refresh icon to show the new cached icon
        await refreshIcon(program)

        ElMessage.success(t('icon_management.reset_cache_success'))
    } catch (e) {
        console.error('Failed to reset cache', e)
        ElMessage.error(t('icon_management.reset_cache_failed') + `: ${e}`)
    }
}

// Initial load
handleSearch()
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.content-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.settings-form {
    max-width: 800px;
    flex-shrink: 0;
}

.el-question-icon {
    margin-left: 8px;
    color: #909399;
    cursor: help;
}

.custom-icon-section {
    margin-top: 20px;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 400px;
}

.table-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
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

.path-text {
    font-size: 12px;
    color: #909399;
    word-break: break-all;
}

.action-buttons {
    display: flex;
    gap: 8px;
}
</style>
