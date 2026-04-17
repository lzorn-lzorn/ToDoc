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

        // 检查文件级导出标记 ---<!export> 或 ---<!export ModuleName>（忽略前导空行）。
        let (has_export_marker, module_name) = parse_export_marker(&lines);

        let overview = parse_file_overview(&lines);
        let dependencies = parse_dependencies(&lines);
        let mut apis = Vec::new();

        let raw_abs = std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string();
        // Windows canonicalize 会加 \\?\ 前缀，vscode URI 协议不认。
        let abs_path = raw_abs.strip_prefix(r"\\?\").unwrap_or(&raw_abs).to_string();

        let mut comment_buffer: Vec<String> = Vec::new();
        // 跟踪函数嵌套深度：只在顶层（depth == 0）收集 API。
        let mut nesting_depth: usize = 0;
        // 块注释状态。
        let mut in_block_comment = false;
        let mut block_close_pattern = String::new();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // ── 块注释内部 ──
            if in_block_comment {
                if let Some(close_pos) = trimmed.find(block_close_pattern.as_str()) {
                    let content = &trimmed[..close_pos];
                    if nesting_depth == 0 && !content.trim().is_empty() {
                        comment_buffer.push(content.trim().to_string());
                    }
                    in_block_comment = false;
                } else if nesting_depth == 0 {
                    comment_buffer.push(trimmed.to_string());
                }
                continue;
            }

            // ── 块注释开始 ──
            if let Some((content, close_pat)) = try_open_block_comment(trimmed) {
                block_close_pattern = close_pat.clone();
                if let Some(close_pos) = content.find(close_pat.as_str()) {
                    // 同行关闭。
                    let inner = &content[..close_pos];
                    if nesting_depth == 0 && !inner.trim().is_empty() {
                        comment_buffer.push(inner.trim().to_string());
                    }
                } else {
                    if nesting_depth == 0 && !content.trim().is_empty() {
                        comment_buffer.push(content.trim().to_string());
                    }
                    in_block_comment = true;
                }
                continue;
            }

            // 统计嵌套深度（简化方式：逐行匹配 Lua block 关键字）。
            let depth_delta = count_block_deltas(trimmed);

            if let Some(comment_text) = strip_lua_comment_prefix(line) {
                if nesting_depth == 0 {
                    comment_buffer.push(comment_text);
                }
                // 注释行不改变嵌套深度。
                continue;
            }

            if nesting_depth == 0 {
                if let Some((name, func_type, table_name, signature)) = parse_function_signature(trimmed) {
                    join_continuation_lines(&mut comment_buffer);
                    let raw_comment = comment_buffer.join("\n");
                    let parsed = parse_comment(&raw_comment, config);
                    // @private 显式禁止导出，优先级最高。
                    // local function 默认不导出，除非显式 @export。
                    // TableMethod / Global 只在有标签或显式 @export 时导出。
                    let exported = if parsed.private {
                        false
                    } else {
                        match func_type {
                            FuncType::Local => parsed.exported,
                            _ => parsed.exported || parsed.has_tags,
                        }
                    };
                    apis.push(ApiDoc {
                        name,
                        signature,
                        func_type,
                        table_name,
                        line_number: idx + 1,
                        file_path: abs_path.clone(),
                        exported,
                        brief: parsed.brief,
                        params: parsed.params,
                        returns: parsed.returns,
                        notes: parsed.notes,
                        usages: parsed.usages,
                        deprecated: parsed.deprecated,
                        todos: parsed.todos,
                        raw_comment: parsed.raw_comment,
                    });
                    comment_buffer.clear();
                } else if !trimmed.is_empty() {
                    // 非注释且非函数定义时，清空缓存，避免跨段错误绑定。
                    comment_buffer.clear();
                }
            }

            nesting_depth = (nesting_depth as isize + depth_delta) .max(0) as usize;
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
            file_exported: has_export_marker,
            module_name,
        })
    }

    fn file_extensions(&self) -> &[&str] {
        &["lua"]
    }
}

/// 解析文件导出标记 `---<!export>` 或 `---<!export ModuleName>`。
/// 返回 (是否有导出标记, 模块名)。无模块名时默认 `"Global"`。
fn parse_export_marker(lines: &[&str]) -> (bool, String) {
    let first_non_blank = lines.iter().filter(|l| !l.trim().is_empty()).next();
    match first_non_blank {
        Some(line) => {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("---<!export") {
                let rest = rest.trim_end_matches('>');
                let name = rest.trim();
                if name.is_empty() {
                    (true, "Global".to_string())
                } else {
                    (true, name.to_string())
                }
            } else {
                (false, "Global".to_string())
            }
        }
        None => (false, "Global".to_string()),
    }
}

/// 解析文件开头连续注释为概述（忽略中间空行，遇到代码才停止）。
fn parse_file_overview(lines: &[&str]) -> String {
    let mut overview = Vec::new();
    let mut in_block_comment = false;
    let mut block_close_pattern = String::new();

    for line in lines {
        let trimmed = line.trim();

        // ── 块注释内部 ──
        if in_block_comment {
            if let Some(close_pos) = trimmed.find(block_close_pattern.as_str()) {
                let content = &trimmed[..close_pos];
                if !content.trim().is_empty() {
                    overview.push(content.trim().to_string());
                }
                in_block_comment = false;
            } else {
                overview.push(trimmed.to_string());
            }
            continue;
        }

        // ── 块注释开始 ──
        if let Some((content, close_pat)) = try_open_block_comment(trimmed) {
            block_close_pattern = close_pat.clone();
            if let Some(close_pos) = content.find(close_pat.as_str()) {
                let inner = &content[..close_pos];
                if !inner.trim().is_empty() {
                    overview.push(inner.trim().to_string());
                }
            } else {
                if !content.trim().is_empty() {
                    overview.push(content.trim().to_string());
                }
                in_block_comment = true;
            }
            continue;
        }

        // ── 空行：跳过，不中断概述 ──
        if trimmed.is_empty() {
            if overview.is_empty() {
                continue;
            }
            overview.push(String::new());
            continue;
        }

        // ── 单行注释 ──
        if let Some(text) = strip_lua_comment_prefix(line) {
            // 跳过导出标记本身（含可选模块名）。
            if text.trim().starts_with("<!export") {
                continue;
            }
            overview.push(text);
        } else {
            // 遇到代码，结束概述。
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

/// 合并延续行：`@` 后紧跟空白（不是有效标签名）的行视为上一行的延续。
/// 这在 Lua 注释中常见于为了对齐或编辑器着色而使用 `-- @      延续内容`。
fn join_continuation_lines(buffer: &mut Vec<String>) {
    let mut i = 1;
    while i < buffer.len() {
        let trimmed = buffer[i].trim_start().to_string();
        let is_continuation = match trimmed.strip_prefix('@') {
            Some(rest) => rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t'),
            None => false,
        };
        if is_continuation {
            let content = trimmed
                .strip_prefix('@')
                .unwrap_or("")
                .trim_start()
                .to_string();
            if !content.is_empty() {
                buffer[i - 1].push('\n');
                buffer[i - 1].push_str(&content);
            }
            buffer.remove(i);
        } else {
            i += 1;
        }
    }
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
    // 排除块注释开头（由 try_open_block_comment 处理）。
    let after_dashes = &trimmed[dash_count..];
    if after_dashes.starts_with('[') {
        let eq_count = after_dashes[1..].chars().take_while(|c| *c == '=').count();
        if after_dashes.as_bytes().get(1 + eq_count) == Some(&b'[') {
            return None;
        }
    }
    let rest = after_dashes;
    Some(rest.trim_start().to_string())
}

/// 尝试解析 Lua 块注释开头（`--[[` 或 `--[=[`...）。
/// 返回 (开始标记后的同行内容, 结束标记模式)。
fn try_open_block_comment(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    // 至少需要 --[[
    if !trimmed.starts_with("--[") {
        return None;
    }
    let after_dashes = &trimmed[3..]; // 跳过 "--["
    // 统计 '=' 的数量。
    let eq_count = after_dashes.chars().take_while(|c| *c == '=').count();
    // 验证第二个 '['。
    if after_dashes.as_bytes().get(eq_count) != Some(&b'[') {
        return None;
    }
    let open_suffix_len = eq_count + 1; // =*[
    let content = after_dashes[open_suffix_len..].to_string();
    let close = format!("]{}]", "=".repeat(eq_count));
    Some((content, close))
}

/// 解析 Lua 函数定义，返回 (函数名, 类型, 所属表名, 完整签名)。
fn parse_function_signature(line: &str) -> Option<(String, FuncType, Option<String>, String)> {
    let s = line.trim();

    // 提取从 "function" 到行尾右括号的完整签名。
    let extract_signature = |prefix: &str| -> String {
        // 找到完整的 function xxx(...)
        if let Some(paren_end) = s.find(')') {
            s[..=paren_end].to_string()
        } else {
            prefix.to_string()
        }
    };

    if let Some(rest) = s.strip_prefix("local function ") {
        let name = read_identifier_like(rest)?;
        let signature = extract_signature(&format!("local function {}", name));
        return Some((name, FuncType::Local, None, signature));
    }

    if let Some(rest) = s.strip_prefix("function ") {
        let symbol = read_identifier_like(rest)?;
        let signature = extract_signature(&format!("function {}", symbol));
        if let Some((table, method)) = symbol.split_once('.') {
            return Some((method.to_string(), FuncType::TableMethod, Some(table.to_string()), signature));
        }
        if let Some((table, method)) = symbol.split_once(':') {
            return Some((method.to_string(), FuncType::TableMethod, Some(table.to_string()), signature));
        }
        return Some((symbol, FuncType::Global, None, signature));
    }

    None
}

/// 统计一行中 Lua block 关键字带来的嵌套深度变化。
fn count_block_deltas(line: &str) -> isize {
    let trimmed = line.trim();
    // 忽略注释行。
    if trimmed.starts_with("--") {
        return 0;
    }

    let mut delta: isize = 0;

    // 去掉字符串字面量以避免误匹配。
    let cleaned = strip_lua_strings(trimmed);

    // 统计 block 开始关键字。
    for word in cleaned.split(|c: char| !c.is_alphanumeric() && c != '_') {
        match word {
            "function" | "if" | "for" | "while" | "repeat" => delta += 1,
            "end" => delta -= 1,
            "until" => delta -= 1, // repeat..until
            _ => {}
        }
    }

    delta
}

/// 简单去除 Lua 字符串内容以避免关键字误匹配。
fn strip_lua_strings(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            result.push(' ');
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
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
