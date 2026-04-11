use crate::core::image_processor::ImageIdentity;
use crate::core::image_processor::ImageProcessor;
use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::ui_config::PartialUiConfig;
use crate::modules::everything::config::PartialEverythingConfig;
use crate::modules::shortcut_manager::shortcut_config::PartialShortcutConfig;
use crate::state::app_state::AppState;
use crate::utils::service_locator::ServiceLocator;
use crate::utils::ui_controller::handle_focus_lost;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;
use tracing::info;

#[tauri::command]
pub async fn update_search_bar_window<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<
    (
        PartialAppConfig,
        PartialUiConfig,
        PartialShortcutConfig,
        PartialEverythingConfig,
    ),
    String,
> {
    let runtime_config = state.get_runtime_config();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    let shortcut_config = runtime_config.get_shortcut_config();
    let everything_config = runtime_config.get_everything_config();
    Ok((
        app_config.to_partial(),
        ui_config.to_partial(),
        shortcut_config.to_partial(),
        everything_config.to_partial(),
    ))
}

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let storage_manager = state.get_storage_manager();
    if let Some(data) = storage_manager
        .download_file_bytes("background.png".to_string())
        .await
    {
        return Ok(data);
    } else {
        storage_manager
            .upload_file_bytes("background.png".to_string(), Vec::new())
            .await;
    }
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_remote_config_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let storage_manager = state.get_storage_manager();
    let path = storage_manager.get_target_dir_path().await;
    Ok(path)
}

#[tauri::command]
pub async fn select_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let path = ImageIdentity::File(path);
    let content: Vec<u8> = ImageProcessor::load_image(&path).await;
    let storage_manager = state.get_storage_manager();
    storage_manager
        .upload_file_bytes("background.png".to_string(), content)
        .await;
    if let Err(e) = app.emit("update_search_bar_window", "") {
        return Err(format!("Failed to emit update event: {:?}", e));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_dominant_color<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<String, String> {
    let path = ImageIdentity::File(path);
    let content = ImageProcessor::load_image(&path).await;
    let ret = match ImageProcessor::get_dominant_color(content).await {
        Ok(color) => color,
        Err(e) => return Err(format!("Failed to get dominant color: {:?}", e)),
    };
    Ok(format!("rgba({}, {}, {}, 0.8)", ret.0, ret.1, ret.2))
}

#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn get_everything_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<Vec<u8>, String> {
    let icon_manager = state.get_icon_manager();
    let icon_data = icon_manager.get_everything_icon(path).await;
    Ok(icon_data)
}

#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn get_everything_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _state: tauri::State<'_, Arc<AppState>>,
    _path: String,
) -> Result<Vec<u8>, String> {
    Ok(Vec::new())
}

/// 隐藏窗口
#[tauri::command]
pub fn hide_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let main_window = match state.get_main_handle().get_webview_window("main") {
        Some(window) => window,
        None => return Err("Failed to get main window".to_string()),
    };
    handle_focus_lost(Arc::new(main_window));
    Ok(())
}

/// 展示设置窗口
#[tauri::command]
pub fn show_setting_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let setting_window = match state.get_main_handle().get_webview_window("setting_window") {
        Some(window) => window,
        None => return Err("Failed to get setting window".to_string()),
    };

    // 从配置中读取保存的窗口位置和大小
    let config = state.get_runtime_config();
    let setting_window_state = config.get_setting_window_state();
    let saved_x = setting_window_state.get_window_x();
    let saved_y = setting_window_state.get_window_y();
    let saved_width = setting_window_state.get_window_width();
    let saved_height = setting_window_state.get_window_height();

    // 获取主监视器信息
    if let Some(monitor) = setting_window
        .primary_monitor()
        .map_err(|e| e.to_string())?
    {
        let monitor_size = monitor.size();
        let monitor_position = monitor.position();

        // 智能调整窗口大小：如果窗口大小超过屏幕的80%，则缩小
        let max_width = (monitor_size.width as f64 * 0.8) as u32;
        let max_height = (monitor_size.height as f64 * 0.8) as u32;
        let mut adjusted_width = saved_width;
        let mut adjusted_height = saved_height;

        if saved_width > max_width || saved_height > max_height {
            // 保持宽高比缩放
            let width_ratio = max_width as f64 / saved_width as f64;
            let height_ratio = max_height as f64 / saved_height as f64;
            let scale = width_ratio.min(height_ratio).min(1.0);

            adjusted_width = (saved_width as f64 * scale) as u32;
            adjusted_height = (saved_height as f64 * scale) as u32;

            // 确保最小尺寸
            adjusted_width = adjusted_width.max(800);
            adjusted_height = adjusted_height.max(500);

            info!(
                "Window size {}x{} exceeds 80% of screen, adjusted to {}x{}",
                saved_width, saved_height, adjusted_width, adjusted_height
            );
        }

        // 检查窗口是否超出屏幕边界或为默认位置
        let mut should_center = false;

        // 如果位置是默认的 (0, 0)，则居中
        if saved_x == 0 && saved_y == 0 {
            should_center = true;
            info!("Setting window position is default (0,0), will center it");
        }
        // 检查窗口是否在屏幕可视区域内（使用调整后的大小）
        else if saved_x < monitor_position.x
            || saved_y < monitor_position.y
            || saved_x + adjusted_width as i32 > monitor_position.x + monitor_size.width as i32
            || saved_y + adjusted_height as i32 > monitor_position.y + monitor_size.height as i32
        {
            should_center = true;
            info!(
                "Setting window is out of screen bounds, will center it. Saved position: x={}, y={}, size: {}x{}, Monitor: position=({}, {}), size={}x{}",
                saved_x, saved_y, saved_width, saved_height,
                monitor_position.x, monitor_position.y,
                monitor_size.width, monitor_size.height
            );
        }

        if should_center {
            // 计算居中位置（使用调整后的大小）
            let centered_x =
                monitor_position.x + (monitor_size.width as i32 - adjusted_width as i32) / 2;
            let centered_y =
                monitor_position.y + (monitor_size.height as i32 - adjusted_height as i32) / 2;

            // 应用居中位置
            use tauri::PhysicalPosition;
            setting_window
                .set_position(PhysicalPosition::new(centered_x, centered_y))
                .map_err(|e| format!("Failed to set window position: {:?}", e))?;

            info!(
                "Centered setting window at: x={}, y={}",
                centered_x, centered_y
            );
        } else {
            // 使用保存的位置
            use tauri::PhysicalPosition;
            setting_window
                .set_position(PhysicalPosition::new(saved_x, saved_y))
                .map_err(|e| format!("Failed to set window position: {:?}", e))?;
        }

        // 应用调整后的窗口大小（使用物理像素）
        use tauri::PhysicalSize;
        setting_window
            .set_size(PhysicalSize::new(adjusted_width, adjusted_height))
            .map_err(|e| format!("Failed to set window size: {:?}", e))?;
    }

    let _ = setting_window.unminimize();
    if let Err(e) = setting_window.show() {
        return Err(format!("Failed to show setting window: {:?}", e));
    }

    // 发送窗口显示事件，通知前端重置加载状态
    if let Err(e) = setting_window.emit("window-shown", ()) {
        tracing::warn!("Failed to emit window-shown event: {:?}", e);
    }

    if let Err(e) = setting_window.set_focus() {
        return Err(format!("Failed to set focus on setting window: {:?}", e));
    }
    if let Err(e) = hide_window() {
        return Err(format!("Failed to hide window: {:?}", e));
    }
    Ok(())
}

/// 显示欢迎窗口
#[tauri::command]
pub async fn show_welcome_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    use std::sync::Arc;
    use tauri::{LogicalSize, WebviewUrl, WebviewWindowBuilder};

    // 先关闭已存在的欢迎窗口（如果有的话）
    if let Some(existing_window) = app.get_webview_window("welcome") {
        let _ = existing_window.close();
    }

    // 创建新的欢迎窗口
    let welcome_result =
        WebviewWindowBuilder::new(&app, "welcome", WebviewUrl::App("/welcome".into()))
            .title("欢迎使用 ZeroLaunch-rs!")
            .visible(true)
            .drag_and_drop(false)
            .build();

    match welcome_result {
        Ok(welcome_window) => {
            if let Err(e) = welcome_window.set_size(LogicalSize::new(950, 500)) {
                return Err(format!("Failed to set welcome window size: {:?}", e));
            }

            // 监听窗口关闭事件，确保窗口关闭时清除内存
            let welcome_arc = Arc::new(welcome_window);
            let welcome_for_event = welcome_arc.clone();
            welcome_for_event.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    // 窗口关闭时，窗口会自动从内存中清除
                    // 这里可以添加额外的清理逻辑（如果需要的话）
                }
            });
        }
        Err(e) => {
            return Err(format!("Failed to create welcome window: {:?}", e));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn command_is_system_dark_mode<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        match window.theme() {
            Ok(theme) => Ok(theme == tauri::Theme::Dark),
            Err(_) => Ok(false),
        }
    } else {
        Ok(false)
    }
}
