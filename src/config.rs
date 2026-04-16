//! 全局配置模块，负责读取和管理 todoc.json 配置。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;

/// ToDoc 的全局配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 缓存目录。
    pub cache_dir: String,
    /// HTML 输出目录。
    pub output_dir: String,
    /// 自动包装 \content 时使用的默认格式。
    pub default_format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_dir: ".todoc_cache".to_string(),
            output_dir: "docs".to_string(),
            default_format: "markdown".to_string(),
        }
    }
}

impl Config {
    /// 从配置文件读取配置；如果文件不存在则返回默认配置。
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        let config = serde_json::from_str::<Self>(&text)?;
        Ok(config)
    }
}
