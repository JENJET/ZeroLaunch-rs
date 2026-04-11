use std::fmt::Debug;

use super::config::PartialLocalConfig;
use super::config::StorageDestination;
// use super::onedrive::OneDriveStorage;
use super::webdav::WebDAVStorage;
use crate::core::storage::config::LocalConfig;
use crate::core::storage::local_save::LocalStorage;
use crate::core::storage::utils::create_str;
use crate::core::storage::utils::read_str;
use crate::error::{AppError, AppResult};
use crate::utils::notify::notify;
use crate::LOCAL_CONFIG_PATH;
use async_trait::async_trait;
use dashmap::DashMap;
use dashmap::Entry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub const TEST_CONFIG_FILE_NAME: &str = "zerolaunch-test-link.txt";
pub const TEST_CONFIG_FILE_DATA: &str = "当前文件仅用于测试连通性，可以手动删除";
pub const WELCOME_PAGE_VERSION: &str = "1.0.1";
/// 存储管理器的配置文件为 appdata下的目录，这个决定了远程配置文件保存的位置
#[async_trait]
pub trait StorageClient: Send + Sync {
    // 要可以上传文件
    async fn upload(&self, file_name: String, data: Vec<u8>) -> AppResult<()>;
    // 要可以下载文件
    async fn download(&self, file_name: String) -> AppResult<Option<Vec<u8>>>;
    // 要可以删除文件
    async fn delete(&self, file_name: String) -> AppResult<()>;
    // 要可以获得当前文件的目标路径
    async fn get_target_dir_path(&self) -> String;
    // 判断是否有效(true: 有效，false: 无效)
    async fn validate_config(&self) -> bool;
}

pub struct StorageManagerInner {
    /// 当前的存储信息
    pub local_config: RwLock<LocalConfig>,
    /// 缓存的数据(文件名, (剩余更新次数, 要上传的内容))
    pub cached_content: DashMap<String, (u32, Vec<u8>)>,
    /// 上传文件与下载文件的对象
    pub client: RwLock<Option<Arc<dyn StorageClient>>>,
}

impl std::fmt::Debug for StorageManagerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageManagerInner")
            .field("local_config", &self.local_config)
            .field("cached_content", &self.cached_content)
            .finish()
    }
}

impl StorageManagerInner {
    // 创建一个存储管理器
    // callback：当检测到版本更新时（说明用户做了更新），或者没有配置文件时（说明用户第一次启动程序），调用该函数

    pub async fn new<F>(callback: F) -> StorageManagerInner
    where
        F: Fn(),
    {
        let inner = StorageManagerInner {
            local_config: RwLock::new(LocalConfig::default()),
            cached_content: DashMap::new(),
            client: RwLock::new(None),
        };

        // 直接读取本地的配置文件，如果读取失败了，则说明是用户第一次启动程序，需要调用callback函数
        let result = read_str(&LOCAL_CONFIG_PATH);

        let mut is_first_startup = false;
        let local_config_data = match result {
            Err(error) => {
                // 从本地读取配置信息，这个default_content就是当用户读取本地配置信息失败时，要写入的初始值
                let default_content =
                    match serde_json::to_string(&inner.local_config.read().await.to_partial()) {
                        Ok(content) => content,
                        Err(e) => {
                            error!("Failed to serialize default local config: {}", e);
                            // 使用硬编码的默认配置作为后备
                            "{}".to_string()
                        }
                    };

                if error.kind() == std::io::ErrorKind::NotFound {
                    // 如果没有这个文件，则说明是用户第一次启动程序
                    is_first_startup = true;
                    // 写入初始值
                    if let Err(e) = create_str(&LOCAL_CONFIG_PATH, &default_content) {
                        warn!("创建本地配置文件失败: {}", e);
                    } else {
                        debug!("Created initial local config file");
                    }
                } else {
                    warn!("读取本地配置文件失败: {}", error);
                }
                default_content
            }
            Ok(local_config_data) => {
                debug!("Successfully loaded local config file");
                local_config_data
            }
        };
        debug!(
            "Local config data loaded: {} bytes",
            local_config_data.len()
        );

        let partial_local_config: PartialLocalConfig =
            match serde_json::from_str(&local_config_data) {
                Ok(config) => {
                    debug!("Successfully parsed local config");
                    config
                }
                Err(e) => {
                    error!("Failed to parse local config: {}, using default", e);
                    // 使用默认配置
                    PartialLocalConfig::default()
                }
            };

        // 检查是否需要显示欢迎页面
        // 首次启动或欢迎页面版本更新时显示欢迎页面
        let should_show_welcome =
            is_first_startup || check_welcome_page_version_changed(&partial_local_config);

        if should_show_welcome {
            callback();
        }

        inner.update_and_refresh(partial_local_config).await;
        inner
    }
    /// 获得当前的本地配置文件的信息
    pub async fn to_partial(&self) -> PartialLocalConfig {
        self.local_config.read().await.to_partial()
    }

    // 更新配置并刷新后端
    pub async fn update_and_refresh(&self, partial_local_config: PartialLocalConfig) {
        {
            let mut local_config = self.local_config.write().await;
            local_config.update(partial_local_config);
            // 根据配置信息选择合理的后端
            let mut client = self.client.write().await;
            match *local_config.get_storage_destination() {
                StorageDestination::Local => {
                    *client = Some(Arc::new(LocalStorage::new(
                        local_config.get_local_save_config(),
                    )));
                }
                StorageDestination::WebDAV => {
                    *client = Some(Arc::new(WebDAVStorage::new(
                        local_config.get_webdav_save_config(),
                    )));
                }
                // StorageDestination::OneDrive => {
                //     self.client = Some(Arc::new(RwLock::new(
                //         OneDriveStorage::new(self.local_config.get_onedrive_save_config()).await,
                //     )))
                // }
                _ => {}
            }
        }
        // 由于后端可能因安全需要而更改配置（比如onedrive），所以要生成以后再保存配置文件
        self.save_to_local_disk().await;
    }

    // 将自己的信息保存到本地
    async fn save_to_local_disk(&self) {
        let partial_local_config = self.local_config.read().await.to_partial();

        let contents = match serde_json::to_string_pretty(&partial_local_config) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to serialize local config for saving: {}", e);
                return;
            }
        };

        let path = LOCAL_CONFIG_PATH.clone();
        if let Err(e) = tokio::fs::write(&path, contents).await {
            error!("Failed to save local config to disk: {}", e);
        } else {
            debug!("Successfully saved local config to disk");
        }
    }

    /// 上传文件
    /// file_name: 工作目录下的相对地址
    /// contents: 内容
    pub async fn upload_file_str(&self, file_name: String, contents: String) -> bool {
        self.upload_file_bytes(file_name, contents.into_bytes())
            .await
    }

    /// 下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_str(&self, file_name: String) -> Option<String> {
        let bytes = self.download_file_bytes(file_name).await?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }
    /// 强制下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_str_force(&mut self, file_name: String) -> Option<String> {
        let bytes = self.download_file_bytes_force(file_name).await?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// 上传文件
    /// file_name: 工作目录下的相对地址
    /// contents: 内容
    pub async fn upload_file_bytes(&self, file_name: String, contents: Vec<u8>) -> bool {
        info!(
            "📤 开始上传文件: {}, 大小: {} bytes",
            file_name,
            contents.len()
        );

        let save_count = *self
            .local_config
            .read()
            .await
            .get_save_to_local_per_update();
        // 若配置为0，直接上传
        if save_count == 0 {
            debug!("⚡ 配置为直接上传模式: {}", file_name);
            return self
                .upload_file_bytes_force(file_name, Some(contents))
                .await;
        }

        match self.cached_content.entry(file_name.clone()) {
            Entry::Occupied(mut entry) => {
                let (counter, data) = entry.get_mut();
                *counter -= 1;
                *data = contents.clone();
                debug!("🔄 更新缓存文件: {}, 剩余计数: {}", file_name, *counter);

                if *counter == 0 {
                    // 如果减成了0，则上传文件，同时删除当前的文件
                    debug!("🚀 计数归零，触发上传: {}", file_name);
                    self.upload(file_name.clone(), contents).await;

                    entry.remove();
                }
            }
            Entry::Vacant(entry) => {
                debug!("➕ 添加新缓存文件: {}, 初始计数: {}", file_name, save_count);
                entry.insert((save_count, contents));
            }
        }
        info!("✅ 文件上传操作完成: {}", file_name);
        true
    }

    /// 强制上传文件, 忽略之前的文件
    /// 如果contents有内容，则直接发送该内容，否则，直接发送缓存的内容
    pub async fn upload_file_bytes_force(
        &self,
        file_name: String,
        mut contents: Option<Vec<u8>>,
    ) -> bool {
        match self.cached_content.entry(file_name.clone()) {
            Entry::Occupied(entry) => {
                if contents.is_none() {
                    let (_, data) = entry.get();
                    contents = Some(data.clone())
                }
                entry.remove();
            }
            Entry::Vacant(_) => {
                // 如果没有内容，则忽略
            }
        }
        if let Some(data) = contents {
            self.upload(file_name, data).await;
            return true;
        }
        false
    }

    /// 将当前缓存中所有的文件都上传
    pub async fn upload_all_file_force(&self) {
        // 收集所有需要上传的键值对
        let items_to_upload: Vec<(String, Vec<u8>)> = self
            .cached_content
            .iter()
            .map(|item| (item.key().clone(), item.value().1.clone()))
            .collect();

        // 上传所有文件
        for (key, value) in items_to_upload {
            self.upload(key, value).await;
        }

        // 上传完成后清空缓存
        self.cached_content.clear();
    }

    /// 强制下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_bytes_force(&mut self, file_name: String) -> Option<Vec<u8>> {
        match self.cached_content.entry(file_name.clone()) {
            Entry::Occupied(entry) => {
                // 如果有文件，则删除对应的文件
                entry.remove();
            }
            Entry::Vacant(_) => {
                // 如果没有内容，则忽略
            }
        }

        self.download(file_name).await
    }

    /// 下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_bytes(&self, file_name: String) -> Option<Vec<u8>> {
        info!("📥 开始下载文件: {}", file_name);

        let cached_data = self
            .cached_content
            .get(&file_name)
            .map(|entry| entry.value().1.clone());

        if let Some(content) = cached_data {
            debug!(
                "💾 从缓存获取文件: {}, 大小: {} bytes",
                file_name,
                content.len()
            );
            return Some(content);
        }

        debug!("🌐 从远程下载文件: {}", file_name);
        let result = self.download(file_name.clone()).await;

        match &result {
            Some(data) => info!("✅ 文件下载完成: {}, 大小: {} bytes", file_name, data.len()),
            None => warn!("❌ 文件下载失败: {}", file_name),
        }

        result
    }

    /// 获得目标文件夹的地址
    pub async fn get_target_dir_path(&self) -> String {
        let client_lock = self.client.read().await;
        match client_lock.as_ref() {
            Some(client) => client.get_target_dir_path().await,
            None => {
                error!("存储客户端未初始化，无法获取目标文件夹路径");
                String::new() // 或者返回一个默认路径
            }
        }
    }

    /// 下载文件(写在这里，方便以后做错误处理)
    async fn download(&self, file_name: String) -> Option<Vec<u8>> {
        let result = {
            let client_lock = self.client.read().await;
            match client_lock.as_ref() {
                Some(client) => client.download(file_name.clone()).await,
                None => {
                    warn!("存储客户端未初始化，无法下载文件：{}", file_name);
                    notify(
                        "zerolaunch-rs",
                        &format!(
                            "下载文件：{} 失败，客户端未成功初始化，已切换回默认配置",
                            file_name,
                        ),
                    );
                    Err(AppError::NetworkError {
                        message: "存储客户端未初始化，无法下载文件".to_string(),
                        source: None,
                    })
                }
            }
        };

        match result {
            Ok(data) => {
                if data.is_some() {
                    debug!("成功下载文件：{}", file_name);
                } else {
                    debug!("文件不存在：{}", file_name);
                }
                data
            }
            Err(e) => {
                warn!(
                    "下载文件：{} 失败，已使用默认配置信息，错误信息：{}",
                    file_name,
                    e.to_string()
                );
                notify(
                    "zerolaunch-rs",
                    &format!(
                        "下载文件：{} 失败，错误：{:?}，已切换回默认配置",
                        file_name, e
                    ),
                );
                let local_config = LocalConfig::default();
                self.update_and_refresh(local_config.to_partial()).await;

                // 递归调用自身重试下载
                Box::pin(self.download(file_name)).await
            }
        }
    }

    /// 上传文件(写在这里，方便以后做错误处理)
    async fn upload(&self, file_name: String, contents: Vec<u8>) {
        let result = {
            let client_lock = self.client.read().await;
            match client_lock.as_ref() {
                Some(client) => client.upload(file_name.clone(), contents.clone()).await,
                None => {
                    warn!("存储客户端未初始化，无法上传文件：{}", file_name);
                    notify(
                        "zerolaunch-rs",
                        &format!("存储客户端未初始化，无法上传文件：{}", file_name),
                    );
                    Err(AppError::NetworkError {
                        message: "存储客户端未初始化，无法上传文件".to_string(),
                        source: None,
                    })
                }
            }
        };

        match result {
            Ok(_) => {
                info!("成功上传文件：{}", file_name);
            }
            Err(e) => {
                warn!("上传文件：{} 失败，错误：{:?}", file_name, e);
                notify(
                    "zerolaunch-rs",
                    &format!(
                        "上传文件：{} 失败，错误：{:?}，已切换回默认配置",
                        file_name, e
                    ),
                );
                let local_config = LocalConfig::default();
                self.update_and_refresh(local_config.to_partial()).await;
                Box::pin(self.upload(file_name, contents)).await
            }
        }
    }
}
#[derive(Debug)]
pub struct StorageManager {
    pub inner: RwLock<StorageManagerInner>,
}

impl StorageManager {
    /// 创建一个新的 StorageManager 实例
    pub async fn new<F>(callback: F) -> Self
    where
        F: Fn(),
    {
        Self {
            inner: RwLock::new(StorageManagerInner::new(callback).await),
        }
    }

    /// 获得当前的本地配置文件的信息
    pub async fn to_partial(&self) -> PartialLocalConfig {
        let inner = self.inner.read().await;
        inner.to_partial().await
    }

    /// 更新存储管理器配置
    pub async fn update(&self, partial_local_config: PartialLocalConfig) {
        let inner = self.inner.write().await;
        inner.update_and_refresh(partial_local_config).await
    }

    /// 上传字符串内容到指定文件（带缓存策略）
    pub async fn upload_file_str(&self, file_name: String, contents: String) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_str(file_name, contents).await
    }

    /// 下载文件内容为字符串（优先使用缓存）
    pub async fn download_file_str(&self, file_name: String) -> Option<String> {
        let inner = self.inner.write().await;
        inner.download_file_str(file_name).await
    }

    /// 下载文件内容为字符串
    pub async fn download_file_str_force(&self, file_name: String) -> Option<String> {
        let mut inner = self.inner.write().await;
        inner.download_file_str_force(file_name).await
    }

    /// 上传二进制内容到指定文件（带缓存策略）
    pub async fn upload_file_bytes(&self, file_name: String, contents: Vec<u8>) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_bytes(file_name, contents).await
    }

    /// 下载文件内容为二进制（优先使用缓存）
    pub async fn download_file_bytes(&self, file_name: String) -> Option<Vec<u8>> {
        let inner = self.inner.write().await;
        inner.download_file_bytes(file_name).await
    }

    /// 下载文件内容为二进行（强制下载）
    pub async fn download_file_bytes_force(&self, file_name: String) -> Option<Vec<u8>> {
        let mut inner = self.inner.write().await;
        inner.download_file_bytes_force(file_name).await
    }

    /// 强制上传文件内容（绕过缓存策略）
    pub async fn upload_file_bytes_force(
        &self,
        file_name: String,
        contents: Option<Vec<u8>>,
    ) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_bytes_force(file_name, contents).await
    }

    /// 强制上传所有缓存中的内容
    pub async fn upload_all_file_force(&self) {
        let inner = self.inner.read().await;
        inner.upload_all_file_force().await;
    }

    /// 获得目标文件夹的路径
    pub async fn get_target_dir_path(&self) -> String {
        let inner = self.inner.read().await;
        inner.get_target_dir_path().await
    }
}

// 检测配置是不是有效的
pub async fn check_validation(
    partial_local_config: PartialLocalConfig,
) -> Option<PartialLocalConfig> {
    let mut config = LocalConfig::default();
    config.update(partial_local_config);
    let client: Option<Arc<dyn StorageClient>> = match *config.get_storage_destination() {
        StorageDestination::Local => {
            let client = Arc::new(LocalStorage::new(config.get_local_save_config()));
            Some(client)
        }
        StorageDestination::WebDAV => {
            let client = Arc::new(WebDAVStorage::new(config.get_webdav_save_config()));
            Some(client)
        }
        // StorageDestination::OneDrive => {
        //     println!(
        //         "当前onedrive的配置: {:?}",
        //         config.get_onedrive_save_config()
        //     );
        //     let client = Arc::new(OneDriveStorage::new(config.get_onedrive_save_config()).await);
        //     println!("已成功赋值onedrive");
        //     Some(client)
        // }
        _ => None,
    };

    if let Some(client) = client.as_ref() {
        if client.validate_config().await {
            // 如果有效，则返回经过修改的PartialLocalConfig
            Some(config.to_partial())
        } else {
            None
        }
    } else {
        None
    }
}

/// 检查welcome页面版本是否发生变化
fn check_welcome_page_version_changed(partial_local_config: &PartialLocalConfig) -> bool {
    // 获取当前welcome页面版本
    let current_welcome_version = get_current_welcome_page_version();

    // 获取存储的welcome页面版本
    let stored_welcome_version = partial_local_config.welcome_page_version.as_ref();

    // 如果没有存储版本或版本不匹配，则需要显示welcome页面
    match stored_welcome_version {
        None => true, // 没有存储版本，需要显示
        Some(stored_version) => stored_version != &current_welcome_version, // 版本不匹配，需要显示
    }
}

/// 获取当前welcome页面版本
fn get_current_welcome_page_version() -> String {
    WELCOME_PAGE_VERSION.to_string()
}
