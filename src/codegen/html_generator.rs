//! HTML 生成器：将 IR 渲染为文件页与索引页，按源码目录树组织输出。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{html, Options, Parser};

use crate::codegen::themes;
use crate::ir::{ApiDoc, FileDoc};
use crate::Result;

/// HTML 生成器。
#[derive(Debug)]
pub struct HtmlGenerator {
    /// 当前使用的主题名。
    pub theme: String,
}

impl Default for HtmlGenerator {
    fn default() -> Self {
        Self { theme: "default".to_string() }
    }
}

impl HtmlGenerator {
    /// 生成完整站点（文件页 + 各级 index.html），保持源码目录结构。
    pub fn generate_site(
        &self,
        docs: &[FileDoc],
        output_dir: &Path,
        source_root: &Path,
        readme_files: &[PathBuf],
    ) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        // 收集源码目录树中所有子目录（相对路径），使空目录也能出现。
        let mut all_dirs: BTreeSet<PathBuf> = BTreeSet::new();
        all_dirs.insert(PathBuf::new()); // 根目录
        collect_all_subdirs(source_root, source_root, &mut all_dirs)?;

        // 按相对目录分组文档。
        let mut dir_docs: BTreeMap<PathBuf, Vec<&FileDoc>> = BTreeMap::new();

        for doc in docs {
            let rel = relative_path(&doc.file_path, source_root);
            let parent = rel.parent().unwrap_or(Path::new("")).to_path_buf();
            dir_docs.entry(parent).or_default().push(doc);
        }

        // 收集所有有文档子项的子目录集合（用于 index 中列出子目录）。
        let mut dir_subdirs: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
        for dir in &all_dirs {
            if let Some(parent) = dir.parent() {
                if all_dirs.contains(parent) {
                    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                        dir_subdirs
                            .entry(parent.to_path_buf())
                            .or_default()
                            .insert(name.to_string());
                    }
                }
            }
        }

        // 为每个目录生成文件页和 index.html。
        for dir in &all_dirs {
            let out_subdir = output_dir.join(dir);
            fs::create_dir_all(&out_subdir)?;

            // 生成该目录下的文件页。
            if let Some(file_docs) = dir_docs.get(dir) {
                for doc in file_docs {
                    let file_name = source_file_to_html_name(&doc.file_path);
                    let back_link = "index.html".to_string();
                    let page = self.render_file_page(doc, &back_link);
                    fs::write(out_subdir.join(&file_name), page)?;
                }
            }

            // 生成 index.html。
            let subdirs: Vec<&str> = dir_subdirs
                .get(dir)
                .map(|s| s.iter().map(|n| n.as_str()).collect())
                .unwrap_or_default();
            let file_docs: Vec<&&FileDoc> = dir_docs
                .get(dir)
                .map(|v| v.iter().collect())
                .unwrap_or_default();
            let is_root = dir.as_os_str().is_empty();
            let dir_display = if is_root {
                "根目录".to_string()
            } else {
                dir.to_string_lossy().to_string()
            };
            let index = self.render_index_page(&dir_display, is_root, &subdirs, &file_docs);
            fs::write(out_subdir.join("index.html"), index)?;
        }

        // 复制 README.md 文件（包裹为可浏览器渲染的 HTML）。
        for readme_path in readme_files {
            let rel = relative_path(
                &readme_path.to_string_lossy(),
                source_root,
            );
            let out_path = output_dir.join(&rel);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // 读取 markdown 内容，生成自包含 HTML 页面。
            let md_content = fs::read_to_string(readme_path).unwrap_or_default();
            if md_content.trim().is_empty() {
                // 空 README 也复制一个最小页面。
                let page = wrap_readme_html("README", "<p>（空文件）</p>", &self.theme);
                fs::write(out_path, page)?;
            } else {
                let rendered = render_markdown(&md_content);
                let page = wrap_readme_html("README", &rendered, &self.theme);
                fs::write(out_path, page)?;
            }
        }

        Ok(())
    }

    /// 渲染单文件页面。
    fn render_file_page(&self, doc: &FileDoc, back_link: &str) -> String {
        let css = themes::css_for_theme(&self.theme);
        let mut html_out = String::new();
        html_out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>ToDoc</title>");
        html_out.push_str("<style>");
        html_out.push_str(css);
        html_out.push_str("</style></head><body><div class=\"container\">");

        // 标题只显示文件名。
        let display_name = Path::new(&doc.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&doc.file_path);
        html_out.push_str(&format!("<h1>{}</h1>", escape_html(display_name)));
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
            if !api.exported {
                continue;
            }
            html_out.push_str(&self.render_api_card(api));
        }

        html_out.push_str(&format!(
            "<p><a href=\"{}\">返回索引</a></p>",
            escape_html(back_link)
        ));
        html_out.push_str("</div></body></html>");
        html_out
    }

    /// 渲染索引页面。
    fn render_index_page(
        &self,
        dir_display: &str,
        is_root: bool,
        subdirs: &[&str],
        docs: &[&&FileDoc],
    ) -> String {
        let css = themes::css_for_theme(&self.theme);
        let mut html_out = String::new();
        html_out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>ToDoc Index</title>");
        html_out.push_str("<style>");
        html_out.push_str(css);
        html_out.push_str("</style></head><body><div class=\"container\">");
        html_out.push_str(&format!(
            "<h1>ToDoc - {}</h1>",
            escape_html(dir_display)
        ));

        // 返回上级目录。
        if !is_root {
            html_out.push_str("<p><a href=\"../index.html\">...</a></p>");
        }

        // 子目录列表。
        if !subdirs.is_empty() {
            html_out.push_str("<div class=\"card\"><h2>子目录</h2><ul>");
            for sub in subdirs {
                html_out.push_str(&format!(
                    "<li>\u{1F4C1} <a href=\"{}/index.html\">{}/</a></li>",
                    escape_html(sub),
                    escape_html(sub)
                ));
            }
            html_out.push_str("</ul></div>");
        }

        // 文件列表。
        if !docs.is_empty() {
            html_out.push_str("<div class=\"card\"><h2>文件</h2><ul>");
            for doc in docs {
                let html_name = source_file_to_html_name(&doc.file_path);
                let display = Path::new(&doc.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&doc.file_path);
                let exported_count = doc.apis.iter().filter(|a| a.exported).count();
                html_out.push_str(&format!(
                    "<li><a href=\"{}\">{}</a> <span class=\"meta\">({} APIs)</span></li>",
                    escape_html(&html_name),
                    escape_html(display),
                    exported_count
                ));
            }
            html_out.push_str("</ul></div>");
        }

        html_out.push_str("</div></body></html>");
        html_out
    }

    /// 渲染单个 API 卡片。
    fn render_api_card(&self, api: &ApiDoc) -> String {
        use crate::ir::FuncType;

        let mut out = String::new();
        let location_link = format!("vscode://file/{}:{}", api.file_path, api.line_number);

        out.push_str("<div class=\"card\">");
        out.push_str(&format!(
            "<h3><a href=\"{}\">{}</a></h3>",
            escape_html(&location_link),
            escape_html(&api.signature)
        ));

        let ownership = match &api.func_type {
            FuncType::TableMethod => {
                if let Some(table) = &api.table_name {
                    format!("所属: {}", table)
                } else {
                    "表方法".to_string()
                }
            }
            FuncType::Global => "全局函数".to_string(),
            FuncType::Local => "局部函数".to_string(),
        };
        out.push_str(&format!(
            "<p class=\"meta\">行号: {} | {} | 导出: {}</p>",
            api.line_number, escape_html(&ownership), api.exported
        ));

        if let Some(brief) = &api.brief {
            out.push_str(&render_markdown(brief));
        }

        if !api.params.is_empty() {
            out.push_str("<h4 class=\"param-heading\">参数</h4><ul>");
            for p in &api.params {
                out.push_str("<li>");
                out.push_str(&format!("<span class=\"param-name\">{}</span>", escape_html(&p.name)));
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
            out.push_str("<h4>备注</h4>");
            for note in &api.notes {
                out.push_str(&format!("<p class=\"note-text\">{}</p>", render_markdown_inline(note)));
            }
        }

        if api.deprecated {
            out.push_str("<p class=\"deprecated\">该 API 已废弃</p>");
        }

        if !api.todos.is_empty() {
            out.push_str("<h4 class=\"todo-heading\">TODO</h4><ul>");
            for todo in &api.todos {
                out.push_str(&format!("<li class=\"todo\">{}</li>", render_markdown_inline(todo)));
            }
            out.push_str("</ul>");
        }

        out.push_str("</div>");
        out
    }
}

/// 递归收集源码目录树中所有子目录的相对路径。
fn collect_all_subdirs(
    root: &Path,
    current: &Path,
    dirs: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(rel) = path.strip_prefix(root) {
                dirs.insert(rel.to_path_buf());
            }
            collect_all_subdirs(root, &path, dirs)?;
        }
    }
    Ok(())
}

/// 从文件路径计算相对于源码根目录的相对路径。
fn relative_path(file_path: &str, source_root: &Path) -> PathBuf {
    let fp = Path::new(file_path);
    // 尝试 strip 源码根目录前缀。
    if let Ok(rel) = fp.strip_prefix(source_root) {
        return rel.to_path_buf();
    }
    // 如果是规范化路径，也试试规范化的根。
    if let Ok(canonical_root) = source_root.canonicalize() {
        let clean_root = canonical_root
            .to_string_lossy()
            .strip_prefix(r"\\?\")
            .unwrap_or(&canonical_root.to_string_lossy())
            .to_string();
        if let Some(rest) = file_path.strip_prefix(&clean_root) {
            let trimmed = rest.trim_start_matches(['/', '\\']);
            return PathBuf::from(trimmed);
        }
    }
    // 回退：只取文件名。
    PathBuf::from(fp.file_name().unwrap_or(fp.as_os_str()))
}

/// 将源文件名映射为 HTML 文件名（只用文件名部分）。
fn source_file_to_html_name(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    format!("{}.html", name)
}

/// 包装 README 渲染结果为自包含 HTML 页面。
fn wrap_readme_html(title: &str, body_html: &str, theme: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title}</title>
<style>{css}</style></head><body><div class="container">
<p><a href="index.html">返回索引</a></p>
{body}
</div></body></html>"#,
        title = escape_html(title),
        css = themes::css_for_theme(theme),
        body = body_html,
    )
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

/// 最小化 HTML 转义。
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
