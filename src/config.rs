//! 全局配置模块，负责读取和管理 todoc.json 配置。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// 源文件清单：按目录分组或单列文件。
/// JSON 示例：
/// ```json
/// {
///   "dirA": [],          // 空数组 = 目录A全量扫描
///   "dirB": ["a.lua", "b.lua"]  // 指定文件
/// }
/// ```
pub type SourceEntries = BTreeMap<String, Vec<String>>;

/// ToDoc 的全局配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 工作区根路径。所有相对路径均以此为基准。
    #[serde(default = "default_workspace")]
    pub workspace: String,
    /// 缓存目录。
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// 文档输出目录。
    #[serde(default = "default_doc_target_out_dir")]
    pub doc_target_out_dir: String,
    /// 自动包装 \content 时使用的默认格式。
    #[serde(default = "default_format")]
    pub default_format: String,
    /// 源文件清单：key 为目录路径，value 为该目录下的文件列表（空列表表示全量扫描）。
    #[serde(default)]
    pub sources: SourceEntries,
    /// 源码根路径（默认为 workspace 路径）。
    #[serde(default = "default_workspace")]
    pub source_code: String,
    /// 输出格式（例如 `"html"`）。
    #[serde(default = "default_target_code")]
    pub target_code: String,
    /// 主题名称。
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_workspace() -> String {
    ".".to_string()
}

fn default_cache_dir() -> String {
    ".todoc/cache".to_string()
}

fn default_doc_target_out_dir() -> String {
    ".todoc/docs".to_string()
}

fn default_format() -> String {
    "markdown".to_string()
}

fn default_target_code() -> String {
    "html".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: ".".to_string(),
            cache_dir: ".todoc/cache".to_string(),
            doc_target_out_dir: ".todoc/docs".to_string(),
            default_format: "markdown".to_string(),
            sources: BTreeMap::new(),
            source_code: ".".to_string(),
            target_code: "html".to_string(),
            theme: "default".to_string(),
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

/// 将路径解析为绝对路径：若 `path` 是绝对路径则直接返回，否则相对于 `base` 解析。
pub fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}
