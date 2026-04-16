//! 多语言解析器抽象。

use std::path::Path;

use crate::config::Config;
use crate::ir::FileDoc;
use crate::Result;

pub mod lua;

/// 语言解析器统一接口。
pub trait LanguageParser {
    /// 解析一个源代码文件并输出 FileDoc。
    fn parse_file(&self, path: &Path, config: &Config) -> Result<FileDoc>;
    /// 支持的文件扩展名。
    fn file_extensions(&self) -> &[&str];
}
