//! Token 类型定义。

/// 注释词法 Token。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `@brief`、`@param`、`@return`、`@note`、`@deprecated`、`@todo`、`@export`
    Tag(String),
    /// `\\type{int}`、`\\content[markdown]{...}`、`\\default{0}`、`\\name{xxx}`
    KeywordLabel {
        name: String,
        format: Option<String>,
        content: String,
    },
    /// 普通文本。
    Text(String),
    /// 换行。
    Newline,
    /// 结束标记。
    Eof,
}
