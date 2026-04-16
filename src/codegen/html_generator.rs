//! HTML 生成器：将 IR 渲染为文件页与索引页。

use std::fs;
use std::path::Path;

use pulldown_cmark::{html, Options, Parser};

use crate::codegen::themes::soft;
use crate::ir::{ApiDoc, FileDoc};
use crate::Result;

/// HTML 生成器。
#[derive(Debug, Default)]
pub struct HtmlGenerator;

impl HtmlGenerator {
    /// 生成完整站点（文件页 + 索引页）。
    pub fn generate_site(&self, docs: &[FileDoc], output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        for doc in docs {
            let file_name = file_doc_to_html_name(&doc.file_path);
            let page = self.render_file_page(doc);
            fs::write(output_dir.join(file_name), page)?;
        }

        let index = self.render_index_page(docs);
        fs::write(output_dir.join("index.html"), index)?;
        Ok(())
    }

    /// 渲染单文件页面。
    fn render_file_page(&self, doc: &FileDoc) -> String {
        let mut html_out = String::new();
        html_out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>ToDoc</title>");
        html_out.push_str("<style>");
        html_out.push_str(soft::css());
        html_out.push_str("</style></head><body><div class=\"container\">");

        html_out.push_str(&format!("<h1>{}</h1>", escape_html(&doc.file_path)));
        html_out.push_str("<div class=\"card\"><h2>文件概述</h2>");
        html_out.push_str(&render_markdown(&doc.overview));
        html_out.push_str("</div>");

        if !doc.dependencies.is_empty() {
            html_out.push_str("<div class=\"card\"><h2>依赖</h2><ul>");
            for dep in &doc.dependencies {
                html_out.push_str(&format!("<li><code>{}</code></li>", escape_html(dep)));
            }
            html_out.push_str("</ul></div>");
        }

        html_out.push_str("<h2>API</h2>");
        for api in &doc.apis {
            html_out.push_str(&self.render_api_card(api));
        }

        html_out.push_str("<p><a href=\"index.html\">返回索引</a></p>");
        html_out.push_str("</div></body></html>");
        html_out
    }

    /// 渲染索引页面。
    fn render_index_page(&self, docs: &[FileDoc]) -> String {
        let mut html_out = String::new();
        html_out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>ToDoc Index</title>");
        html_out.push_str("<style>");
        html_out.push_str(soft::css());
        html_out.push_str("</style></head><body><div class=\"container\">");
        html_out.push_str("<h1>ToDoc 文档索引</h1><div class=\"card\"><ul>");

        for doc in docs {
            let file_name = file_doc_to_html_name(&doc.file_path);
            html_out.push_str(&format!(
                "<li><a href=\"{}\">{}</a> <span class=\"meta\">({} APIs)</span></li>",
                escape_html(&file_name),
                escape_html(&doc.file_path),
                doc.apis.len()
            ));
        }

        html_out.push_str("</ul></div></div></body></html>");
        html_out
    }

    /// 渲染单个 API 卡片。
    fn render_api_card(&self, api: &ApiDoc) -> String {
        let mut out = String::new();
        let location_link = format!("vscode://file/{}:{}", api.file_path, api.line_number);

        out.push_str("<div class=\"card\">");
        out.push_str(&format!(
            "<h3><a href=\"{}\">{}</a></h3>",
            escape_html(&location_link),
            escape_html(&api.name)
        ));
        out.push_str(&format!(
            "<p class=\"meta\">行号: {} | 导出: {}</p>",
            api.line_number, api.exported
        ));

        if let Some(brief) = &api.brief {
            out.push_str(&render_markdown(brief));
        }

        if !api.params.is_empty() {
            out.push_str("<h4>参数</h4><ul>");
            for p in &api.params {
                out.push_str("<li>");
                out.push_str(&format!("<strong>{}</strong>", escape_html(&p.name)));
                if let Some(t) = &p.type_name {
                    out.push_str(&format!(" <code>{}</code>", escape_html(t)));
                }
                if let Some(d) = &p.default_value {
                    out.push_str(&format!(" 默认=<code>{}</code>", escape_html(d)));
                }
                out.push_str(&format!(" - {}", render_markdown_inline(&p.description)));
                out.push_str("</li>");
            }
            out.push_str("</ul>");
        }

        if !api.returns.is_empty() {
            out.push_str("<h4>返回</h4><ul>");
            for r in &api.returns {
                out.push_str("<li>");
                if let Some(t) = &r.type_name {
                    out.push_str(&format!("<code>{}</code> ", escape_html(t)));
                }
                out.push_str(&render_markdown_inline(&r.description));
                out.push_str("</li>");
            }
            out.push_str("</ul>");
        }

        if !api.notes.is_empty() {
            out.push_str("<h4>备注</h4><ul>");
            for note in &api.notes {
                out.push_str(&format!("<li>{}</li>", render_markdown_inline(note)));
            }
            out.push_str("</ul>");
        }

        if api.deprecated {
            out.push_str("<p class=\"deprecated\">该 API 已废弃</p>");
        }

        if !api.todos.is_empty() {
            out.push_str("<h4>TODO</h4><ul>");
            for todo in &api.todos {
                out.push_str(&format!("<li class=\"todo\">{}</li>", render_markdown_inline(todo)));
            }
            out.push_str("</ul>");
        }

        out.push_str("</div>");
        out
    }
}

/// 以 markdown 模式渲染文本。
fn render_markdown(text: &str) -> String {
    let parser = Parser::new_ext(text, Options::all());
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// 渲染简短 markdown 片段并去掉外层段落标签。
fn render_markdown_inline(text: &str) -> String {
    let raw = render_markdown(text);
    raw.replace("<p>", "").replace("</p>\n", "").replace("</p>", "")
}

/// 将文件路径映射为稳定 HTML 文件名。
fn file_doc_to_html_name(path: &str) -> String {
    let mapped = path.replace(['/', '\\', ':'], "_");
    format!("{}.html", mapped)
}

/// 最小化 HTML 转义。
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
