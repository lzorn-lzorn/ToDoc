//! 注释解析器：将 Token 节点转换为结构化文档字段。

use crate::config::Config;
use crate::ir::{ParamDoc, ReturnDoc, UsageDoc};
use crate::lexer::{lex, token::Token};

/// API 注释解析结果。
#[derive(Debug, Clone, Default)]
pub struct ParsedComment {
    pub brief: Option<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Vec<ReturnDoc>,
    pub notes: Vec<String>,
    pub usages: Vec<UsageDoc>,
    pub deprecated: bool,
    pub todos: Vec<String>,
    pub exported: bool,
    pub private: bool,
    pub has_tags: bool,
    pub raw_comment: String,
}

/// 解析注释文本。
pub fn parse_comment(raw_comment: &str, config: &Config) -> ParsedComment {
    let desugared = desugar_tag_lines(raw_comment, &config.default_format);
    let tokens = lex(&desugared);

    let mut result = ParsedComment {
        raw_comment: raw_comment.to_string(),
        ..ParsedComment::default()
    };

    // 每个 @Tag 到下一个 @Tag 之间构成一个节点。
    let mut current_tag: Option<String> = None;
    let mut current_segment: Vec<Token> = Vec::new();

    for token in tokens {
        match token {
            Token::Tag(name) => {
                result.has_tags = true;
                if let Some(tag) = current_tag.take() {
                    apply_tag_segment(&mut result, &tag, &current_segment, config);
                    current_segment.clear();
                } else if !current_segment.is_empty() {
                    // 首个标签前的文本应作为自由描述，不应错误并入第一个标签。
                    let leading_text = extract_plain_text(&current_segment);
                    if !leading_text.is_empty() && result.brief.is_none() {
                        result.brief = Some(leading_text);
                    }
                    current_segment.clear();
                }
                current_tag = Some(normalize_tag_name(&name));
            }
            Token::Eof => {
                if let Some(tag) = current_tag.take() {
                    apply_tag_segment(&mut result, &tag, &current_segment, config);
                }
            }
            other => current_segment.push(other),
        }
    }

    // 如果没有任何标签，将全文当作 brief。
    if result.brief.is_none() && !raw_comment.trim().is_empty() {
        let plain = raw_comment.trim().to_string();
        if !plain.is_empty() {
            result.brief = Some(plain);
        }
    }

    result
}

/// 统一标签名：忽略大小写和可选冒号。
fn normalize_tag_name(name: &str) -> String {
    let normalized = name.trim().trim_end_matches(':').to_ascii_lowercase();
    match normalized.as_str() {
        // 兼容项目中常见拼写。
        "breif" => "brief".to_string(),
        _ => normalized,
    }
}

/// 将一个标签节点的内容应用到解析结果中。
fn apply_tag_segment(result: &mut ParsedComment, tag: &str, segment: &[Token], config: &Config) {
    let detail = parse_segment_detail(segment, &config.default_format);

    match tag {
        "brief" => {
            if !detail.content.is_empty() {
                result.brief = Some(detail.content);
            }
        }
        "param" => {
            let name = detail
                .name
                .or_else(|| detail.plain_name.clone())
                .or_else(|| {
                    detail
                        .content
                        .split_whitespace()
                        .next()
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "unknown".to_string());
            // Description priority:
            //   1) Plain-text description inferred from the @param line (e.g. "购买参数,")
            //      takes precedence over any \content{} value, so that a block-comment
            //      --[[\content{...}]] following the @param line cannot contaminate it.
            //   2) When there is no inline plain description (formal syntax: \name{} \content{}),
            //      fall back to the \content{} value as the description.
            //   3) When neither exists, use the implicit content string.
            let description = match detail.description {
                Some(desc) => desc,
                None => detail.content.trim().to_string(),
            };
            result.params.push(ParamDoc {
                name,
                type_name: detail.type_name,
                default_value: detail.default_value,
                description,
            });
        }
        "return" => {
            result.returns.push(ReturnDoc {
                type_name: detail.type_name,
                description: detail.content,
            });
        }
        "note" => {
            if !detail.content.is_empty() {
                result.notes.push(detail.content);
            }
        }
        "usage" => {
            if !detail.content.is_empty() || detail.path.is_some() || detail.api_name.is_some() {
                result.usages.push(UsageDoc {
                    content: detail.content,
                    path: detail.path,
                    api_name: detail.api_name,
                });
            }
        }
        "deprecated" => {
            result.deprecated = true;
            if !detail.content.is_empty() {
                result.notes.push(format!("Deprecated: {}", detail.content));
            }
        }
        "todo" => {
            if !detail.content.is_empty() {
                result.todos.push(detail.content);
            }
        }
        "export" => {
            result.exported = true;
        }
        "private" => {
            result.private = true;
        }
        _ => {}
    }
}

/// 节点细节。
#[derive(Default)]
struct SegmentDetail {
    name: Option<String>,
    plain_name: Option<String>,
    type_name: Option<String>,
    default_value: Option<String>,
    path: Option<String>,
    api_name: Option<String>,
    has_explicit_content: bool,
    content: String,
    description: Option<String>,
}

/// 解析标签节点中的关键字标签和普通文本。
fn parse_segment_detail(segment: &[Token], _default_format: &str) -> SegmentDetail {
    let mut detail = SegmentDetail::default();
    let mut plain_text = String::new();
    let mut explicit_content: Vec<String> = Vec::new();

    for token in segment {
        match token {
            Token::KeywordLabel {
                name,
                format: _,
                content,
            } => match name.as_str() {
                "name" => detail.name = Some(content.trim().to_string()),
                "type" => detail.type_name = Some(content.trim().to_string()),
                "default" | "defualt" => detail.default_value = Some(content.trim().to_string()),
                "path" => detail.path = Some(content.trim().to_string()),
                "apiname" => detail.api_name = Some(content.trim().to_string()),
                "content" => {
                    detail.has_explicit_content = true;
                    explicit_content.push(content.trim().to_string())
                }
                _ => {}
            },
            Token::Text(text) => plain_text.push_str(text),
            Token::Newline => plain_text.push('\n'),
            _ => {}
        }
    }

    // 无显式 \content 时，自动按默认格式包装普通文本。
    let merged = if explicit_content.is_empty() {
        let auto_content = plain_text.trim().to_string();
        if auto_content.is_empty() {
            String::new()
        } else {
            auto_content
        }
    } else {
        explicit_content.join("\n")
    };

    detail.content = strip_leading_separator(&merged).to_string();

    // 参数标签中：若普通文本以“name description”形式出现，拆分为描述。
    let trimmed_plain = plain_text.trim();
    if !trimmed_plain.is_empty() {
        detail.plain_name = trimmed_plain
            .split_whitespace()
            .next()
            .map(ToString::to_string);
        if let Some((_, desc)) = trimmed_plain.split_once(char::is_whitespace) {
            detail.description = Some(strip_leading_separator(desc.trim()).to_string());
        }
    }

    detail
}

/// 对标签行应用语法糖，将简单文本包装为 `\content[default_format]{...}`。
///
/// 支持的标签：
/// - `@param name [\type type] content` → `@param \name{name} [\type{type}] \content[fmt]{content}`
/// - `@brief content` → `@brief \content[fmt]{content}`
/// - `@note content` → `@note \content[fmt]{content}`
/// - `@todo content` → `@todo \content[fmt]{content}`
/// - `@deprecated content` → `@deprecated \content[fmt]{content}`
///
/// 所有多行延续内容（`@ + 空白` 或普通文本行）用 `\n` 保持换行。
fn desugar_tag_lines(raw: &str, default_format: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut out = String::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim_start();

        // 尝试识别 @tag。
        let (tag_name, body) = match try_strip_any_tag(trimmed) {
            Some(pair) => pair,
            None => {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(lines[i]);
                i += 1;
                continue;
            }
        };

        let tag_lower = tag_name.to_ascii_lowercase();

        if tag_lower == "param" {
            // ── @param 语法糖 ──
            if body.contains(r"\name{") || body.contains(r"\type{") {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(lines[i]);
                i += 1;
                continue;
            }

            // 若后续延续行包含显式 \content{...}，则只吃当前行和 continuation 行，不把 block comment 并入 param。
            if has_explicit_content_continuation(&lines, i + 1) {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(lines[i]);
                i += 1;
                // 只吃连续的 continuation 行，不吃 block comment 或新 tag。
                while i < lines.len() {
                    let next_trimmed = lines[i].trim_start();
                    if next_trimmed.is_empty() {
                        break;
                    }
                    if next_trimmed.starts_with('@') {
                        if let Some(rest) = next_trimmed.strip_prefix('@') {
                            let is_continuation =
                                rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t');
                            if !is_continuation {
                                break;
                            }
                        }
                    }
                    // 若遇到 block comment 或显式 \content，终止 param sugar。
                    if next_trimmed.starts_with("--[[") || next_trimmed.starts_with(r"\\content{") || next_trimmed.starts_with(r"\\content[") {
                        break;
                    }
                    out.push('\n');
                    out.push_str(lines[i]);
                    i += 1;
                }
                continue;
            }

            let (name, type_name, first_content) = parse_sugar_param(body);

            let mut content_parts: Vec<&str> = Vec::new();
            if !first_content.is_empty() {
                content_parts.push(first_content);
            }
            i += 1;
            collect_continuation_lines(&lines, &mut i, &mut content_parts);

            let merged_content = content_parts.join("\n");

            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("@param ");
            out.push_str(&format!(r"\name{{{}}}", name));
            if let Some(ref ty) = type_name {
                out.push_str(&format!(r" \type{{{}}}", ty));
            }
            if !merged_content.is_empty() {
                out.push_str(&format!(r" \content[{}]{{{}}}", default_format, merged_content));
            }
        } else if is_simple_content_tag(&tag_lower) {
            // ── 简单内容标签语法糖：@brief / @note / @todo / @deprecated ──
            if body.contains(r"\content{") || body.contains(r"\content[") {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(lines[i]);
                i += 1;
                continue;
            }

            let mut content_parts: Vec<&str> = Vec::new();
            if !body.is_empty() {
                content_parts.push(body);
            }
            i += 1;
            collect_continuation_lines(&lines, &mut i, &mut content_parts);

            let merged_content = content_parts.join("\n");

            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("@{}", tag_name));
            if !merged_content.is_empty() {
                out.push_str(&format!(r" \content[{}]{{{}}}", default_format, merged_content));
            }
        } else {
            // 其他标签：直接透传。
            if i > 0 {
                out.push('\n');
            }
            out.push_str(lines[i]);
            i += 1;
        }
    }

    out
}

/// 判断是否为简单内容标签（整个 body 视为 content）。
fn is_simple_content_tag(tag_lower: &str) -> bool {
    matches!(tag_lower, "brief" | "breif" | "note" | "todo" | "deprecated" | "usage")
}

/// 从 trimmed 行中提取 `@tag` 名称和标签后的内容。
fn try_strip_any_tag(trimmed: &str) -> Option<(&str, &str)> {
    if !trimmed.starts_with('@') {
        return None;
    }
    let after_at = &trimmed[1..];
    let tag_end = after_at
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(after_at.len());
    if tag_end == 0 {
        return None;
    }
    let tag_name = &after_at[..tag_end];
    let after = &after_at[tag_end..];
    let after = after.strip_prefix(':').unwrap_or(after);
    Some((tag_name, after.trim_start()))
}

/// 收集延续行（不以新 @tag 开头的后续行），用 `\n` 保持换行。
fn collect_continuation_lines<'a>(lines: &[&'a str], i: &mut usize, parts: &mut Vec<&'a str>) {
    while *i < lines.len() {
        let next_trimmed = lines[*i].trim_start();
        if next_trimmed.is_empty() {
            break;
        }
        if next_trimmed.starts_with('@') {
            if let Some(rest) = next_trimmed.strip_prefix('@') {
                let is_continuation =
                    rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t');
                if is_continuation {
                    let cont = rest.trim_start();
                    if !cont.is_empty() {
                        parts.push(cont);
                    }
                    *i += 1;
                    continue;
                }
            }
            break;
        }
        parts.push(next_trimmed);
        *i += 1;
    }
}

fn has_explicit_content_continuation(lines: &[&str], mut i: usize) -> bool {
    while i < lines.len() {
        let next_trimmed = lines[i].trim_start();
        if next_trimmed.is_empty() {
            break;
        }
        if next_trimmed.starts_with('@') {
            if let Some(rest) = next_trimmed.strip_prefix('@') {
                let is_continuation =
                    rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t');
                if !is_continuation {
                    break;
                }
            }
        }
        if next_trimmed.starts_with(r"\content{") || next_trimmed.starts_with(r"\content[") {
            return true;
        }
        i += 1;
    }
    false
}

/// 从语法糖体中提取 (name, Option<type>, remaining_content)。
///
/// 格式：`param_name [\type param_type] content...`
fn parse_sugar_param(body: &str) -> (&str, Option<&str>, &str) {
    let body = body.trim_start();
    if body.is_empty() {
        return ("unknown", None, "");
    }

    // 提取 param_name（第一个非空白 token，去除尾部逗号）。
    let name_end = body
        .find(|c: char| c.is_whitespace())
        .unwrap_or(body.len());
    let raw_name = &body[..name_end];
    let name = raw_name.trim_end_matches(',');
    let rest = body[name_end..].trim_start();

    // 检查是否有 `\type param_type`。
    if let Some(after_type) = rest.strip_prefix(r"\type") {
        let after_type = after_type.trim_start();
        // 提取 type 名称（到下一个空白）。
        let type_end = after_type
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_type.len());
        let type_name = &after_type[..type_end];
        let content = after_type[type_end..].trim_start();
        (name, Some(type_name), content)
    } else {
        (name, None, rest)
    }
}

fn extract_plain_text(segment: &[Token]) -> String {
    let mut plain = String::new();
    for token in segment {
        match token {
            Token::Text(text) => plain.push_str(text),
            Token::Newline => plain.push('\n'),
            _ => {}
        }
    }
    plain.trim().to_string()
}

fn strip_leading_separator(input: &str) -> &str {
    input
        .trim_start()
        .trim_start_matches(':')
        .trim_start()
}
