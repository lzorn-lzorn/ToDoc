//! ToDoc 的核心库入口，暴露词法、语法、语言解析、IR、缓存和代码生成功能。

pub mod cache;
pub mod codegen;
pub mod config;
pub mod ir;
pub mod lang;
pub mod lexer;
pub mod parser;

/// 项目统一错误类型，便于在各模块间传递错误。
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
