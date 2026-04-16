//! 主题模块导出。

pub mod default;
pub mod soft;

/// 根据主题名返回对应的 CSS 文本。
pub fn css_for_theme(theme: &str) -> &'static str {
    match theme {
        "soft" => soft::css(),
        _ => default::css(),
    }
}
