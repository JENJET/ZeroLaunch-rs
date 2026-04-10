use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 设置窗口状态的部分更新结构
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialSettingWindowState {
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

/// 设置窗口状态配置
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SettingWindowStateInner {
    /// 窗口X坐标
    #[serde(default = "SettingWindowStateInner::default_window_x")]
    pub window_x: i32,
    /// 窗口Y坐标
    #[serde(default = "SettingWindowStateInner::default_window_y")]
    pub window_y: i32,
    /// 窗口宽度
    #[serde(default = "SettingWindowStateInner::default_window_width")]
    pub window_width: u32,
    /// 窗口高度
    #[serde(default = "SettingWindowStateInner::default_window_height")]
    pub window_height: u32,
}

impl Default for SettingWindowStateInner {
    fn default() -> Self {
        Self {
            window_x: Self::default_window_x(),
            window_y: Self::default_window_y(),
            window_width: Self::default_window_width(),
            window_height: Self::default_window_height(),
        }
    }
}

impl SettingWindowStateInner {
    pub(crate) fn default_window_x() -> i32 {
        0
    }

    pub(crate) fn default_window_y() -> i32 {
        0
    }

    pub(crate) fn default_window_width() -> u32 {
        2000
    }

    pub(crate) fn default_window_height() -> u32 {
        1200
    }

    /// 更新配置
    pub fn update(&mut self, partial: PartialSettingWindowState) {
        if let Some(x) = partial.window_x {
            self.window_x = x;
        }
        if let Some(y) = partial.window_y {
            self.window_y = y;
        }
        if let Some(width) = partial.window_width {
            self.window_width = width;
        }
        if let Some(height) = partial.window_height {
            self.window_height = height;
        }
    }

    /// 转换为部分更新结构
    pub fn to_partial(&self) -> PartialSettingWindowState {
        PartialSettingWindowState {
            window_x: Some(self.window_x),
            window_y: Some(self.window_y),
            window_width: Some(self.window_width),
            window_height: Some(self.window_height),
        }
    }
}

/// 设置窗口状态配置的包装器
#[derive(Debug)]
pub struct SettingWindowState {
    inner: RwLock<SettingWindowStateInner>,
}

impl Default for SettingWindowState {
    fn default() -> Self {
        SettingWindowState {
            inner: RwLock::new(SettingWindowStateInner::default()),
        }
    }
}

impl SettingWindowState {
    /// 更新配置
    pub fn update(&self, partial: PartialSettingWindowState) {
        let mut inner = self.inner.write();
        inner.update(partial);
    }

    /// 获取窗口X坐标
    pub fn get_window_x(&self) -> i32 {
        let inner = self.inner.read();
        inner.window_x
    }

    /// 获取窗口Y坐标
    pub fn get_window_y(&self) -> i32 {
        let inner = self.inner.read();
        inner.window_y
    }

    /// 获取窗口宽度
    pub fn get_window_width(&self) -> u32 {
        let inner = self.inner.read();
        inner.window_width
    }

    /// 获取窗口高度
    pub fn get_window_height(&self) -> u32 {
        let inner = self.inner.read();
        inner.window_height
    }

    /// 转换为部分更新结构
    pub fn to_partial(&self) -> PartialSettingWindowState {
        let inner = self.inner.read();
        inner.to_partial()
    }
}
