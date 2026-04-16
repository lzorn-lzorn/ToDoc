//! 缓存管理模块：按文件 mtime 进行增量解析。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ir::FileDoc;
use crate::Result;

/// 缓存管理器。
#[derive(Debug)]
pub struct CacheManager {
    cache_path: PathBuf,
    entries: HashMap<String, FileDoc>,
}

impl CacheManager {
    /// 创建缓存管理器并加载已有缓存。
    pub fn new(cache_dir: &Path) -> Result<Self> {
        fs::create_dir_all(cache_dir)?;
        let cache_path = cache_dir.join("cache.json");
        let entries = if cache_path.exists() {
            let text = fs::read_to_string(&cache_path)?;
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(Self { cache_path, entries })
    }

    /// 若缓存存在且 mtime 一致，则返回缓存文档。
    pub fn get_if_fresh(&self, path: &Path, mtime: u64) -> Option<FileDoc> {
        let key = path.to_string_lossy();
        self.entries
            .get(key.as_ref())
            .filter(|doc| doc.last_modified == mtime)
            .cloned()
    }

    /// 更新缓存条目。
    pub fn update(&mut self, doc: FileDoc) {
        self.entries.insert(doc.file_path.clone(), doc);
    }

    /// 保存缓存到磁盘。
    pub fn save(&self) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.cache_path, text)?;
        Ok(())
    }
}
