use std::sync::Arc;
use tauri::webview::WebviewWindow;
use tauri::Emitter;
use tauri::Manager;
use tracing::warn;

use super::service_locator::ServiceLocator;
use super::windows::is_foreground_fullscreen;
use crate::modules::parameter_resolver::SelectionProvider;
use crate::update_window_size_and_position;

pub fn handle_pressed(app_handle: &tauri::AppHandle) {
    // 如果不是全屏情况下才唤醒
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config();
    let app_config = runtime_config.get_app_config();

    if !app_config.get_is_wake_on_fullscreen() && is_foreground_fullscreen() {
        return;
    }

    // 在显示搜索栏之前,先保存当前的前台窗口句柄和选中文本
    // 注意：必须在获取焦点之前捕获这些信息
    let hwnd_value: isize;
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            hwnd_value = hwnd.0 as isize;
            state.set_previous_foreground_window(Some(hwnd_value));
            tracing::debug!("🎯 保存唤醒前的窗口句柄: {}", hwnd_value);
        } else {
            hwnd_value = 0;
            state.set_previous_foreground_window(None);
            tracing::warn!("⚠️ 无法获取唤醒前的窗口句柄");
        }
    }

    // 在窗口句柄捕获后，立即尝试获取选中文本
    // 此时目标窗口仍然是前台窗口，焦点元素应该是正确的
    if hwnd_value != 0 {
        match SelectionProvider::get_value_from_hwnd(hwnd_value) {
            Ok(selection) => {
                if !selection.is_empty() {
                    tracing::debug!("📝 保存唤醒前的选中文本: {} 字符", selection.len());
                    state.set_previous_selection(Some(selection));
                } else {
                    tracing::debug!("📝 唤醒前没有选中文本");
                    state.set_previous_selection(None);
                }
            }
            Err(e) => {
                tracing::debug!("⚠️ 无法获取选中文本: {}", e);
                state.set_previous_selection(None);
            }
        }
    } else {
        state.set_previous_selection(None);
    }

    update_window_size_and_position();

    let main_window = match app_handle.get_webview_window("main") {
        Some(window) => Arc::new(window),
        None => {
            warn!("Failed to get main window");
            return;
        }
    };

    if let Err(e) = main_window.show() {
        warn!("handle_pressed: Failed to show main window: {}", e);
        return;
    }
    tracing::trace!("handle_pressed: window shown");

    if let Err(e) = main_window.set_focus() {
        warn!("handle_pressed: Failed to set focus on main window: {}", e);
        return;
    }
    tracing::trace!("handle_pressed: window focused");

    if let Err(e) = main_window.emit("show_window", ()) {
        warn!("handle_pressed: Failed to emit show_window event: {}", e);
    }
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(true);
}

pub fn handle_focus_lost(main_window: Arc<WebviewWindow>) {
    main_window
        .hide()
        .unwrap_or_else(|e| warn!("无法隐藏窗口：{}", e));
    if let Err(e) = main_window.emit("handle_focus_lost", ()) {
        warn!("Failed to emit handle_focus_lost event: {}", e);
    }
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(false);
}

pub fn toggle_search_bar(app_handle: &tauri::AppHandle) {
    let state = ServiceLocator::get_state();
    if state.get_search_bar_visible() {
        if let Some(window) = app_handle.get_webview_window("main") {
            handle_focus_lost(Arc::new(window));
        }
    } else {
        handle_pressed(app_handle);
    }
}
