//! Lua 语言解析器实现。

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::Config;
use crate::ir::{ApiDoc, FileDoc, FuncType};
use crate::lang::LanguageParser;
use crate::parser::comment_parser::parse_comment;
use crate::Result;

/// Lua 解析器。
#[derive(Debug, Default)]
pub struct LuaParser;

impl LanguageParser for LuaParser {
    fn parse_file(&self, path: &Path, config: &Config) -> Result<FileDoc> {
        let source = fs::read_to_string(path)?;
        let lines: Vec<&str> = source.lines().collect();

        let overview = parse_file_overview(&lines);
        let dependencies = parse_dependencies(&lines);
        let mut apis = Vec::new();

        let mut comment_buffer: Vec<String> = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if let Some(comment_text) = strip_lua_comment_prefix(line) {
                comment_buffer.push(comment_text);
                continue;
            }

            if let Some((name, func_type, table_name)) = parse_function_signature(line) {
                let raw_comment = comment_buffer.join("\n");
                let parsed = parse_comment(&raw_comment, config);
                apis.push(ApiDoc {
                    name,
                    func_type,
                    table_name,
                    line_number: idx + 1,
                    file_path: path.to_string_lossy().to_string(),
                    exported: parsed.exported,
                    brief: parsed.brief,
                    params: parsed.params,
                    returns: parsed.returns,
                    notes: parsed.notes,
                    deprecated: parsed.deprecated,
                    todos: parsed.todos,
                    raw_comment: parsed.raw_comment,
                });
                comment_buffer.clear();
            } else if !line.trim().is_empty() {
                // 非注释且非函数定义时，清空暂存，避免跨段错误绑定。
                comment_buffer.clear();
            }
        }

        let last_modified = fs::metadata(path)?
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        Ok(FileDoc {
            file_path: path.to_string_lossy().to_string(),
            overview,
            dependencies,
            apis,
            last_modified,
        })
    }

    fn file_extensions(&self) -> &[&str] {
        &["lua"]
    }
}

/// 解析文件开头连续注释为概述。
fn parse_file_overview(lines: &[&str]) -> String {
    let mut overview = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() && overview.is_empty() {
            continue;
        }
        if let Some(text) = strip_lua_comment_prefix(line) {
            overview.push(text);
        } else {
            break;
        }
    }

    overview.join("\n").trim().to_string()
}

/// 提取 require("xxx") 或 require 'xxx' 依赖。
fn parse_dependencies(lines: &[&str]) -> Vec<String> {
    let mut deps = Vec::new();

    for line in lines {
        let s = line.trim();
        if let Some(rest) = s.strip_prefix("require(") {
            if let Some(dep) = extract_quoted(rest.trim_end_matches(')')) {
                deps.push(dep);
            }
        } else if let Some(rest) = s.strip_prefix("require ") {
            if let Some(dep) = extract_quoted(rest) {
                deps.push(dep);
            }
        }
    }

    deps
}

/// 解析字符串字面量内容。
fn extract_quoted(text: &str) -> Option<String> {
    let t = text.trim();
    let quote = t.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let mut chars = t[1..].chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == quote {
            return Some(out);
        }
        out.push(c);
    }
    None
}

/// 去掉 Lua 注释前缀：连续多个 '-' 统一视为 '--'。
fn strip_lua_comment_prefix(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return None;
    }
    let dash_count = trimmed.chars().take_while(|c| *c == '-').count();
    if dash_count < 2 {
        return None;
    }
    let rest = &trimmed[dash_count..];
    Some(rest.trim_start().to_string())
}

/// 解析 Lua 函数定义。
fn parse_function_signature(line: &str) -> Option<(String, FuncType, Option<String>)> {
    let s = line.trim();

    if let Some(rest) = s.strip_prefix("local function ") {
        let name = read_identifier_like(rest)?;
        return Some((name, FuncType::Local, None));
    }

    if let Some(rest) = s.strip_prefix("function ") {
        let symbol = read_identifier_like(rest)?;
        if let Some((table, method)) = symbol.split_once('.') {
            return Some((method.to_string(), FuncType::TableMethod, Some(table.to_string())));
        }
        if let Some((table, method)) = symbol.split_once(':') {
            return Some((method.to_string(), FuncType::TableMethod, Some(table.to_string())));
        }
        return Some((symbol, FuncType::Global, None));
    }

    None
}

/// 读取函数名/符号（直到空白或左括号）。
fn read_identifier_like(text: &str) -> Option<String> {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_whitespace() || c == '(' {
            break;
        }
        out.push(c);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
