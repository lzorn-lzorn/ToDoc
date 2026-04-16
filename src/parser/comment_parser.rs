//! 注释解析器：将 Token 节点转换为结构化文档字段。

use crate::config::Config;
use crate::ir::{ParamDoc, ReturnDoc};
use crate::lexer::{lex, token::Token};

/// API 注释解析结果。
#[derive(Debug, Clone, Default)]
pub struct ParsedComment {
    pub brief: Option<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Vec<ReturnDoc>,
    pub notes: Vec<String>,
    pub deprecated: bool,
    pub todos: Vec<String>,
    pub exported: bool,
    pub raw_comment: String,
}

/// 解析注释文本。
pub fn parse_comment(raw_comment: &str, config: &Config) -> ParsedComment {
    let tokens = lex(raw_comment);

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
                if let Some(tag) = current_tag.take() {
                    apply_tag_segment(&mut result, &tag, &current_segment, config);
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
    name.trim().trim_end_matches(':').to_ascii_lowercase()
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
                .or_else(|| {
                    detail
                        .content
                        .split_whitespace()
                        .next()
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "unknown".to_string());
            let description = detail
                .description
                .unwrap_or_else(|| detail.content.trim().to_string());
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
        _ => {}
    }
}

/// 节点细节。
#[derive(Default)]
struct SegmentDetail {
    name: Option<String>,
    type_name: Option<String>,
    default_value: Option<String>,
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
                "default" => detail.default_value = Some(content.trim().to_string()),
                "content" => explicit_content.push(content.trim().to_string()),
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

    detail.content = merged.clone();

    // 参数标签中：若普通文本以“name description”形式出现，拆分为描述。
    let trimmed_plain = plain_text.trim();
    if !trimmed_plain.is_empty() {
        if let Some((_, desc)) = trimmed_plain.split_once(char::is_whitespace) {
            detail.description = Some(desc.trim().to_string());
        }
    }

    detail
}
