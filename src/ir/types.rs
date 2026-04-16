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
    /// 文件是否包含 `---<!export>` 导出标记。
    #[serde(default)]
    pub file_exported: bool,
    /// `---<!export ModuleName>` 中指定的模块名，缺省为 `"Global"`。
    #[serde(default = "default_module_name")]
    pub module_name: String,
}

fn default_module_name() -> String {
    "Global".to_string()
}

/// API 级文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDoc {
    pub name: String,
    pub signature: String,
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

impl ApiDoc {
    /// 返回包含表名的完整 API 名称，例如 `ShopHelper.GetPrice`。
    /// 没有表名时直接返回函数名。
    pub fn qualified_name(&self) -> String {
        match &self.table_name {
            Some(table) => format!("{}.{}", table, self.name),
            None => self.name.clone(),
        }
    }
}
