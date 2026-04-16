//! 文档生成的中间表示（IR）类型定义。

use serde::{Deserialize, Serialize};

/// 文件级文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDoc {
    pub file_path: String,
    pub overview: String,
    pub dependencies: Vec<String>,
    pub apis: Vec<ApiDoc>,
    pub last_modified: u64,
}

/// API 级文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDoc {
    pub name: String,
    pub func_type: FuncType,
    pub table_name: Option<String>,
    pub line_number: usize,
    pub file_path: String,
    pub exported: bool,
    pub brief: Option<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Vec<ReturnDoc>,
    pub notes: Vec<String>,
    pub deprecated: bool,
    pub todos: Vec<String>,
    pub raw_comment: String,
}

/// 参数文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDoc {
    pub name: String,
    pub type_name: Option<String>,
    pub default_value: Option<String>,
    pub description: String,
}

/// 返回值文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnDoc {
    pub type_name: Option<String>,
    pub description: String,
}

/// 函数类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FuncType {
    Local,
    Global,
    TableMethod,
}
