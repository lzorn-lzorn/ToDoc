//! 词法分析模块，将注释文本切分为 Token 序列。

pub mod token;

use token::Token;

/// 将注释文本切分为 Token。
pub fn lex(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '\n' => {
                tokens.push(Token::Newline);
                i += 1;
            }
            '@' => {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                if end > start {
                    let name: String = chars[start..end].iter().collect();
                    tokens.push(Token::Tag(name));
                    i = end;
                } else {
                    tokens.push(Token::Text("@".to_string()));
                    i += 1;
                }
            }
            '\\' => {
                if let Some((token, consumed)) = parse_keyword_label(&chars[i..]) {
                    tokens.push(token);
                    i += consumed;
                } else {
                    tokens.push(Token::Text("\\".to_string()));
                    i += 1;
                }
            }
            _ => {
                let start = i;
                while i < chars.len()
                    && chars[i] != '\n'
                    && chars[i] != '@'
                    && chars[i] != '\\'
                {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                if !text.is_empty() {
                    tokens.push(Token::Text(text));
                }
            }
        }
    }

    tokens.push(Token::Eof);
    tokens
}

/// 解析形如 \name{..} 或 \content[markdown]{..} 的关键字标签。
fn parse_keyword_label(chars: &[char]) -> Option<(Token, usize)> {
    if chars.first().copied()? != '\\' {
        return None;
    }

    let mut i = 1;
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    if i == 1 {
        return None;
    }

    let name: String = chars[1..i].iter().collect();

    let mut format = None;
    if i < chars.len() && chars[i] == '[' {
        i += 1;
        let format_start = i;
        while i < chars.len() && chars[i] != ']' {
            i += 1;
        }
        if i >= chars.len() {
            return None;
        }
        format = Some(chars[format_start..i].iter().collect());
        i += 1;
    }

    if i >= chars.len() || chars[i] != '{' {
        return None;
    }
    i += 1;

    let content_start = i;
    let mut depth = 1;
    while i < chars.len() {
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                let content: String = chars[content_start..i].iter().collect();
                i += 1;
                return Some((
                    Token::KeywordLabel {
                        name,
                        format,
                        content,
                    },
                    i,
                ));
            }
        }
        i += 1;
    }

    None
}
