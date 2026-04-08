use crate::core::image_processor::{ImageIdentity, ImageProcessor};
use crate::core::storage::utils::read_dir_or_create;
use crate::error::OptionExt;
use crate::modules::config::default::ICON_CACHE_DIR;
use crate::modules::icon_manager::config::{IconManagerConfig, RuntimeIconManagerConfig};
use bincode::Encode;
use dashmap::DashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, warn};
pub mod config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconRequest {
    /// 本地文件路径 (exe, lnk, url, ico, png) -> 提取文件图标
    Path(String),
    /// 网址 -> 下载或查找本地域名图标库
    Url(String),
    /// 文件扩展名 (.txt, .doc) -> 获取系统关联图标
    Extension(String),
}

impl IconRequest {
    pub fn get_hash_string(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        let _ = bincode::encode_into_std_write(self, &mut hasher, bincode::config::standard());
        hasher.finalize().to_hex().to_string()
    }
}

#[derive(Debug, Clone)]
struct IconCacheEntry {
    /// 缓存文件最后更新时间
    last_updated: Instant,
}

#[derive(Debug, Clone)]
struct PendingRequest {
    /// 请求开始时间
    #[allow(dead_code)]
    started_at: Instant,
}

#[derive(Debug)]
struct IconManagerInner {
    /// 默认的应用图标的路径
    default_app_icon_path: String,
    /// 默认的网址图片路径
    default_web_icon_path: String,
    /// 已缓存的图标信息 (hash -> 缓存条目)
    cached_icons: DashMap<String, IconCacheEntry>,
    /// 正在处理的请求 (hash -> 请求信息)
    pending_requests: Arc<Mutex<DashMap<String, PendingRequest>>>,
    /// 要不要开启图片缓存
    enable_icon_cache: AtomicBool,
    /// 要不要联网来获取网址的图标
    enable_online: AtomicBool,
    /// 缓存刷新间隔（秒）
    cache_refresh_interval_secs: u64,
}

impl IconManagerInner {
    pub fn new(runtime_config: RuntimeIconManagerConfig) -> Self {
        let mut inner = Self {
            default_app_icon_path: runtime_config.default_app_icon_path,
            default_web_icon_path: runtime_config.default_web_icon_path,
            enable_icon_cache: AtomicBool::new(true),
            enable_online: AtomicBool::new(true),
            cached_icons: DashMap::new(),
            pending_requests: Arc::new(Mutex::new(DashMap::new())),
            cache_refresh_interval_secs: 600, // 默认10分钟
        };
        inner.init();
        inner
    }

    fn init(&mut self) {
        self.cached_icons = self.scan_cached_icons();
    }

    pub fn load_from_config(&self, config: Arc<IconManagerConfig>) {
        self.enable_icon_cache
            .store(config.get_enable_icon_cache(), Ordering::SeqCst);
        self.enable_online
            .store(config.get_enable_online(), Ordering::SeqCst);

        // ✅ 如果启用缓存且缓存为空，在后台异步扫描（不阻塞）
        if self.enable_icon_cache.load(Ordering::SeqCst) && self.cached_icons.is_empty() {
            let cached_icons_clone = self.cached_icons.clone();
            tokio::task::spawn(async move {
                Self::scan_cached_icons_async(cached_icons_clone).await;
            });
        }
    }

    pub async fn get_icon(&self, request: IconRequest) -> Vec<u8> {
        let hash_name = request.get_hash_string() + ".png";

        // 1. 检查是否有缓存（先查内存索引）
        let has_cache_in_memory = self.enable_icon_cache.load(Ordering::SeqCst)
            && self.cached_icons.contains_key(&hash_name);

        if has_cache_in_memory {
            // 1.1 内存索引命中，从磁盘加载缓存
            let cached_data = self.load_from_cache(&hash_name).await;

            // 1.2 检查是否需要后台刷新
            let should_refresh = self.should_refresh_cache(&hash_name);

            if should_refresh {
                // 后台静默更新，不阻塞当前返回
                self.spawn_background_refresh(request.clone(), hash_name.clone());
            }

            return cached_data;
        }

        // 1.3 内存索引未命中，但启用缓存时尝试从磁盘直接读取（容错处理）
        if self.enable_icon_cache.load(Ordering::SeqCst) {
            if let Some(disk_data) = self.try_load_from_disk(&hash_name).await {
                info!(
                    "Cache file found on disk but not in memory index: {}",
                    hash_name
                );

                // 更新内存索引，避免下次重复读磁盘
                self.cached_icons.insert(
                    hash_name.clone(),
                    IconCacheEntry {
                        last_updated: Instant::now(),
                    },
                );

                // 检查是否需要后台刷新
                let should_refresh = self.should_refresh_cache(&hash_name);
                if should_refresh {
                    self.spawn_background_refresh(request.clone(), hash_name.clone());
                }

                return disk_data;
            }
        }

        // 2. 没有缓存，检查是否已有正在进行的相同请求
        {
            let pending = self.pending_requests.lock().await;
            if pending.contains_key(&hash_name) {
                // 队列中已有相同请求，直接返回默认图标，避免重复加载
                warn!(
                    "Duplicate icon request detected for {}, returning default icon",
                    hash_name
                );
                return self.get_default_icon(&request).await;
            }
        }

        // 3. 注册新请求到队列
        {
            let pending = self.pending_requests.lock().await;
            pending.insert(
                hash_name.clone(),
                PendingRequest {
                    started_at: Instant::now(),
                },
            );
        }

        // 4. 处理不同类型的请求（实际加载图标）
        let (mut icon_data, is_default) = match request {
            IconRequest::Path(path) => self.handle_path_request(path).await,
            IconRequest::Url(url) => self.handle_url_request(url).await,
            IconRequest::Extension(ext) => self.handle_extension_request(ext).await,
        };

        // 5. 从队列中移除已完成的请求
        {
            let pending = self.pending_requests.lock().await;
            pending.remove(&hash_name);
        }

        // 裁剪透明白边
        if !icon_data.is_empty() {
            if let Ok(output) = ImageProcessor::trim_transparent_white_border(icon_data.clone()) {
                icon_data = output;
            }
        }

        // 6. 写入缓存
        if self.enable_icon_cache.load(Ordering::SeqCst) && !is_default && !icon_data.is_empty() {
            self.save_to_cache(&hash_name, icon_data.clone()).await;
        }

        icon_data
    }

    async fn handle_path_request(&self, path: String) -> (Vec<u8>, bool) {
        let data = ImageProcessor::load_image(&ImageIdentity::File(path)).await;
        if data.is_empty() {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_app_icon_path.clone(),
            ))
            .await;
            (default_data, true)
        } else {
            (data, false)
        }
    }

    async fn handle_url_request(&self, url: String) -> (Vec<u8>, bool) {
        if !self.enable_online.load(Ordering::SeqCst) {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_web_icon_path.clone(),
            ))
            .await;
            return (default_data, true);
        }

        let data = ImageProcessor::load_image(&ImageIdentity::Web(url)).await;
        if data.is_empty() {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_web_icon_path.clone(),
            ))
            .await;
            (default_data, true)
        } else {
            (data, false)
        }
    }

    async fn handle_extension_request(&self, ext: String) -> (Vec<u8>, bool) {
        let data = ImageProcessor::load_image(&ImageIdentity::Extension(ext)).await;
        if data.is_empty() {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_app_icon_path.clone(),
            ))
            .await;
            (default_data, true)
        } else {
            (data, false)
        }
    }

    /// 从缓存加载图标
    async fn load_from_cache(&self, hash_name: &str) -> Vec<u8> {
        let cached_icon_dir = ICON_CACHE_DIR.clone();
        let icon_path = Path::new(&cached_icon_dir).join(hash_name);
        let identity = ImageIdentity::File(
            icon_path
                .to_str()
                .expect_programming("缓存路径转换为字符串失败")
                .to_string(),
        );
        ImageProcessor::load_image(&identity).await
    }

    /// 尝试从磁盘直接加载缓存（不依赖内存索引）
    /// 用于容错处理：当磁盘有缓存文件但内存索引缺失时
    async fn try_load_from_disk(&self, hash_name: &str) -> Option<Vec<u8>> {
        let cached_icon_dir = ICON_CACHE_DIR.clone();
        let icon_path = Path::new(&cached_icon_dir).join(hash_name);

        // 检查文件是否存在
        if !icon_path.exists() {
            return None;
        }

        // 尝试读取文件
        let identity = ImageIdentity::File(
            icon_path
                .to_str()
                .expect_programming("缓存路径转换为字符串失败")
                .to_string(),
        );

        let data = ImageProcessor::load_image(&identity).await;

        // 如果读取成功且非空，返回数据
        if !data.is_empty() {
            Some(data)
        } else {
            None
        }
    }

    /// 保存图标到缓存（使用临时文件+原子重命名）
    async fn save_to_cache(&self, hash_name: &str, icon_data: Vec<u8>) {
        let cached_icon_dir = ICON_CACHE_DIR.clone();
        let icon_path = Path::new(&cached_icon_dir).join(hash_name);
        let temp_path = icon_path.with_extension("tmp");

        let icon_data_clone = icon_data.clone();
        let hash_name_clone = hash_name.to_string();
        let cached_icons_clone = self.cached_icons.clone();

        tauri::async_runtime::spawn(async move {
            // 先写入临时文件
            if tokio::fs::write(&temp_path, icon_data_clone).await.is_ok() {
                // 原子性重命名
                if tokio::fs::rename(&temp_path, &icon_path).await.is_ok() {
                    // 更新缓存记录
                    cached_icons_clone.insert(
                        hash_name_clone,
                        IconCacheEntry {
                            last_updated: Instant::now(),
                        },
                    );
                }
            }
        });
    }

    /// 检查是否需要刷新缓存
    fn should_refresh_cache(&self, hash_name: &str) -> bool {
        if let Some(entry) = self.cached_icons.get(hash_name) {
            let elapsed = entry.last_updated.elapsed();
            elapsed.as_secs() >= self.cache_refresh_interval_secs
        } else {
            false
        }
    }

    /// 后台静默刷新图标
    fn spawn_background_refresh(&self, request: IconRequest, hash_name: String) {
        let pending_clone = self.pending_requests.clone();
        let cached_icons_clone = self.cached_icons.clone();

        tauri::async_runtime::spawn(async move {
            // 检查是否已有正在进行的刷新请求
            {
                let pending = pending_clone.lock().await;
                if pending.contains_key(&hash_name) {
                    return; // 已有刷新任务，跳过
                }
            }

            // 注册刷新请求
            {
                let pending = pending_clone.lock().await;
                pending.insert(
                    hash_name.clone(),
                    PendingRequest {
                        started_at: Instant::now(),
                    },
                );
            }

            // 执行刷新（不裁剪白边，保持一致性）
            let icon_data = match request {
                IconRequest::Path(path) => {
                    ImageProcessor::load_image(&ImageIdentity::File(path)).await
                }
                IconRequest::Url(url) => ImageProcessor::load_image(&ImageIdentity::Web(url)).await,
                IconRequest::Extension(ext) => {
                    ImageProcessor::load_image(&ImageIdentity::Extension(ext)).await
                }
            };

            // 从队列移除
            {
                let pending = pending_clone.lock().await;
                pending.remove(&hash_name);
            }

            // 如果成功获取到新图标，更新缓存
            if !icon_data.is_empty() {
                let cached_icon_dir = ICON_CACHE_DIR.clone();
                let icon_path = Path::new(&cached_icon_dir).join(&hash_name);
                let temp_path = icon_path.with_extension("tmp");

                if tokio::fs::write(&temp_path, icon_data).await.is_ok()
                    && tokio::fs::rename(&temp_path, &icon_path).await.is_ok()
                {
                    let hash_for_log = hash_name.clone();
                    tracing::debug!("Icon cache refreshed: {}", hash_for_log);

                    cached_icons_clone.insert(
                        hash_name,
                        IconCacheEntry {
                            last_updated: Instant::now(),
                        },
                    );
                }
            }
        });
    }

    /// 获取默认图标
    async fn get_default_icon(&self, request: &IconRequest) -> Vec<u8> {
        match request {
            IconRequest::Path(_) | IconRequest::Extension(_) => {
                ImageProcessor::load_image(&ImageIdentity::File(self.default_app_icon_path.clone()))
                    .await
            }
            IconRequest::Url(_) => {
                ImageProcessor::load_image(&ImageIdentity::File(self.default_web_icon_path.clone()))
                    .await
            }
        }
    }

    fn scan_cached_icons(&self) -> DashMap<String, IconCacheEntry> {
        let result = DashMap::new();
        if !self.enable_icon_cache.load(Ordering::SeqCst) {
            return result;
        }

        let icon_cache_dir_clone = ICON_CACHE_DIR.clone();
        match read_dir_or_create(icon_cache_dir_clone) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy().into_owned();
                    // 只关注 .png 文件
                    if file_name_str.ends_with(".png") {
                        result.insert(
                            file_name_str.clone(),
                            IconCacheEntry {
                                last_updated: Instant::now(), // 初始化为当前时间，后续会从文件系统读取
                            },
                        );
                    }
                }
            }
            Err(e) => warn!("Error reading icon cache directory: {}", e),
        }
        result
    }

    /// ✅ 异步版本：在后台线程中扫描缓存目录（不阻塞主流程）
    async fn scan_cached_icons_async(cached_icons: DashMap<String, IconCacheEntry>) {
        info!("🔍 [图标缓存] 开始异步扫描缓存目录...");
        let start_time = std::time::Instant::now();

        let icon_cache_dir = ICON_CACHE_DIR.clone();

        // 使用 spawn_blocking 避免阻塞 async runtime
        let entries = tokio::task::spawn_blocking(move || read_dir_or_create(icon_cache_dir))
            .await
            .ok()
            .and_then(|r| r.ok());

        if let Some(entries) = entries {
            let mut count = 0;
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy().into_owned();

                if file_name_str.ends_with(".png") {
                    cached_icons.insert(
                        file_name_str,
                        IconCacheEntry {
                            last_updated: Instant::now(),
                        },
                    );
                    count += 1;
                }
            }

            let elapsed = start_time.elapsed();
            info!(
                "✅ [图标缓存] 扫描完成！共 {} 个缓存文件，耗时: {:?}",
                count, elapsed
            );
        } else {
            warn!("❌ [图标缓存] 无法读取缓存目录");
        }
    }

    pub async fn get_everything_icon(&self, path: String) -> Vec<u8> {
        let path_lower = path.to_lowercase();
        let path_obj = Path::new(&path);

        // 1. 文件夹 → 使用缓存（文件夹图标固定，高频复用）
        if path_obj.is_dir() {
            return self
                .get_icon(IconRequest::Extension("folder".to_string()))
                .await;
        }

        // 2. 常见文档/压缩类型 → 使用扩展名缓存（高频复用）
        let common_extensions = [
            ".txt", ".doc", ".docx", ".pdf", ".xls", ".xlsx", ".ppt", ".pptx", ".zip", ".rar",
            ".7z", ".tar", ".gz", ".csv", ".json", ".xml", ".md", ".rtf", ".odt",
        ];

        if let Some(ext) = path_obj.extension().and_then(|e| e.to_str()) {
            let ext_lower = format!(".{}", ext.to_lowercase());

            if common_extensions.contains(&ext_lower.as_str()) {
                // 常见类型：使用缓存
                return self.get_icon(IconRequest::Extension(ext_lower)).await;
            }
        }

        // 3. 可执行文件/快捷方式 → 不缓存（路径唯一，缓存命中率低）
        if path_lower.ends_with(".exe")
            || path_lower.ends_with(".lnk")
            || path_lower.ends_with(".url")
        {
            return ImageProcessor::load_image(&ImageIdentity::File(path)).await;
        }

        // 4. 图片文件 → 直接加载 + resize（image crate 很快，不需要缓存）
        let image_extensions = [
            ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".tiff", ".tif", ".svg",
        ];
        if image_extensions.iter().any(|ext| path_lower.ends_with(ext)) {
            let data = ImageProcessor::load_image(&ImageIdentity::File(path)).await;
            if !data.is_empty() {
                if let Ok(resized) = ImageProcessor::resize_image(data.clone(), 256, 256).await {
                    return resized;
                }
            }
            return data;
        }

        // 5. 其他罕见类型 → 不缓存（避免污染缓存）
        ImageProcessor::load_image(&ImageIdentity::File(path)).await
    }
}

#[derive(Debug)]
pub struct IconManager {
    inner: Arc<IconManagerInner>,
}

impl IconManager {
    pub fn new(config: RuntimeIconManagerConfig) -> Self {
        Self {
            inner: Arc::new(IconManagerInner::new(config)),
        }
    }

    pub async fn load_from_config(&self, config: Arc<IconManagerConfig>) {
        self.inner.load_from_config(config);
    }

    pub async fn get_icon(&self, request: IconRequest) -> Vec<u8> {
        self.inner.get_icon(request).await
    }

    pub async fn update_program_icon_cache(
        &self,
        icon_request: IconRequest,
        new_icon_source: &str,
    ) -> Result<(), String> {
        if !self.inner.enable_icon_cache.load(Ordering::SeqCst) {
            return Err("Icon cache is disabled".to_string());
        }

        // 1. 计算缓存文件名 (Hash)
        let hash_name = icon_request.get_hash_string() + ".png";
        let cached_icon_dir = ICON_CACHE_DIR.clone();
        let target_icon_path = Path::new(&cached_icon_dir).join(&hash_name);

        // 2. 处理新图标源
        // new_icon_source 可能是图片文件，也可能是 exe/lnk
        let identity = ImageIdentity::File(new_icon_source.to_string());
        let mut icon_data = ImageProcessor::load_image(&identity).await;

        if icon_data.is_empty() {
            return Err("Failed to load new icon".to_string());
        }

        // 3. 裁剪透明白边 (保持一致性)
        if let Ok(output) = ImageProcessor::trim_transparent_white_border(icon_data.clone()) {
            icon_data = output;
        }

        // 4. 如果图标过大，则将其等比例的缩小，以提高读取的速度
        if let Ok(resized) = ImageProcessor::resize_image(icon_data.clone(), 256, 256).await {
            icon_data = resized;
        }

        // 5. 覆盖写入缓存
        tokio::fs::write(target_icon_path, icon_data)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn get_everything_icon(&self, path: String) -> Vec<u8> {
        self.inner.get_everything_icon(path).await
    }
}
