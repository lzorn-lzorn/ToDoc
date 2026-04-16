//! ToDoc CLI 入口。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::Parser;

use todoc::cache::CacheManager;
use todoc::codegen::html_generator::HtmlGenerator;
use todoc::config::{resolve_path, Config};
use todoc::ir::FileDoc;
use todoc::lang::lua::LuaParser;
use todoc::lang::LanguageParser;
use todoc::Result;

/// 命令行参数。
#[derive(Debug, Parser)]
#[command(name = "todoc", version, about = "从源码注释生成 API HTML 文档")]
struct Cli {
    /// 要扫描的源码目录（默认当前目录）。
    #[arg(long, default_value = ".")]
    dir: PathBuf,
    /// 输出目录（覆盖配置中的 doc_target_out_dir）。
    #[arg(long)]
    targetout: Option<PathBuf>,
    /// 配置文件路径。
    #[arg(long, default_value = ".todoc/todoc.json")]
    config: PathBuf,
    /// 按模块名列出所有导出 API。模块名在 `---<!export ModuleName>` 中指定。
    #[arg(long, value_name = "MODULE")]
    find: Option<String>,
    /// 生成文档后用默认浏览器打开输出的 index.html。
    #[arg(long)]
    browse: bool,
    /// 仅对指定的单个文件生成文档（相对于 --dir 的路径）。若文件无 `---<!export>` 标记会提示。
    #[arg(long, value_name = "REL_PATH")]
    file: Option<PathBuf>,
    /// 搜索 API。用法：
    ///   --findapi <Module> <Func>        按模块+函数名搜索
    ///   --findapi -g <Func>              搜索全局模块 (Global) 中的 API
    ///   --findapi -f <Func>              跨模块按函数名搜索
    ///   名称支持 `.*` 通配符，加 `-r` 启用正则。
    #[arg(long, num_args = 1.., value_name = "ARGS", allow_hyphen_values = true)]
    findapi: Option<Vec<String>>,
    /// 初始化：创建 .todoc 目录与默认配置，并扫描工作区源文件。
    #[arg(long)]
    init: bool,
    /// 重新扫描 sources 中的文件并更新缓存。
    #[arg(long)]
    refresh: bool,
}

// ─── 入口 ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    // --init 模式：创建 .todoc 目录与默认配置并扫描。
    if cli.init {
        return cmd_init(&cli.config);
    }

    let config_exists = cli.config.exists();
    let mut config = Config::load(&cli.config)?;

    if !config_exists {
        eprintln!(
            "提示：未找到配置文件 {}，使用默认配置（工作区=当前目录，扫描 *.lua，输出到 .todoc/docs/）。",
            cli.config.display()
        );
        eprintln!("      可执行 todoc --init 创建 .todoc 目录并生成默认配置。\n");
    }

    // --dir 覆盖 config.workspace。
    if cli.dir != Path::new(".") {
        config.workspace = cli.dir.to_string_lossy().to_string();
    }

    // 计算工作区绝对路径（相对路径基于 CWD）。
    let workspace = fs::canonicalize(Path::new(&config.workspace))
        .unwrap_or_else(|_| PathBuf::from(&config.workspace));

    // --targetout 覆盖 doc_target_out_dir。
    if let Some(out) = &cli.targetout {
        config.doc_target_out_dir = out.to_string_lossy().to_string();
    }

    // 将配置中的路径解析为基于 workspace 的绝对路径。
    let cache_dir = resolve_path(&workspace, &config.cache_dir);
    let output_dir = resolve_path(&workspace, &config.doc_target_out_dir);

    // 解析 sources 中的路径。
    let resolved_sources = resolve_sources(&workspace, &config.sources);

    // --refresh 模式：重新扫描并更新缓存。
    if cli.refresh {
        return cmd_refresh(&workspace, &cache_dir, &resolved_sources, &config);
    }

    // --findapi 模式。
    if let Some(ref args) = cli.findapi {
        return cmd_findapi(args, &workspace, &cache_dir, &resolved_sources, &config);
    }

    // --find 模式。
    if let Some(ref module) = cli.find {
        return cmd_find(module, &workspace, &cache_dir, &resolved_sources, &config);
    }

    // --file 模式。
    if let Some(ref rel_path) = cli.file {
        return cmd_file(rel_path, &workspace, &cache_dir, &output_dir, &config);
    }

    // 常规文档生成模式。
    cmd_generate(&workspace, &cache_dir, &output_dir, &resolved_sources, &config, cli.browse)
}

// ─── 子命令实现 ──────────────────────────────────────────────────────────────

/// `--init`：创建 .todoc 目录，生成默认配置，扫描工作区。
fn cmd_init(config_path: &Path) -> Result<()> {
    // 如果配置已存在，不覆盖。
    if config_path.exists() {
        eprintln!("配置文件 {} 已存在，跳过初始化。", config_path.display());
        return Ok(());
    }

    // 获取当前 exe 所在目录作为 workspace 和 source_code。
    let exe_dir = std::env::current_exe()?
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let exe_dir_str = display_path(&exe_dir);

    // 创建 .todoc 目录。
    let todoc_dir = Path::new(".todoc");
    fs::create_dir_all(todoc_dir)?;
    println!("已创建目录：{}", todoc_dir.display());

    // 构造配置：workspace 和 source_code 设为当前 exe 路径。
    let mut config = Config::default();
    config.workspace = exe_dir_str.clone();
    config.source_code = exe_dir_str;

    // 扫描工作区，记录到 sources。
    let workspace = fs::canonicalize(Path::new(&config.workspace))
        .unwrap_or_else(|_| PathBuf::from(&config.workspace));
    let parser = LuaParser;
    let exts = parser.file_extensions();
    let files = collect_source_files(&workspace, exts)?;

    if !files.is_empty() {
        // 按相对目录分组到 sources。
        let mut src_map: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        for f in &files {
            if let Ok(rel) = f.strip_prefix(&workspace) {
                let dir_part = rel.parent().unwrap_or(Path::new("."));
                let file_name = rel.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                src_map
                    .entry(dir_part.to_string_lossy().to_string())
                    .or_default()
                    .push(file_name.to_string());
            }
        }
        config.sources = src_map;
    }

    // 写入配置。
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(config_path, json)?;
    println!("已生成配置文件：{}", config_path.display());
    println!("扫描到 {} 个源文件，已写入 sources。", files.len());

    // 扫描并缓存。
    let cache_dir = resolve_path(&workspace, &config.cache_dir);
    let resolved_sources = resolve_sources(&workspace, &config.sources);
    let (all_docs, _) = collect_and_parse(&workspace, &cache_dir, &resolved_sources, &config)?;
    let exported = all_docs.iter().filter(|d| d.file_exported).count();
    println!("预缓存完成：{} 个文件已解析（{} 个导出）。", all_docs.len(), exported);

    Ok(())
}

/// `--refresh`：重新扫描 sources 中的文件并更新缓存。
fn cmd_refresh(
    workspace: &Path,
    cache_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
) -> Result<()> {
    let (all_docs, files_count) = collect_and_parse(workspace, cache_dir, sources, config)?;
    let exported = all_docs.iter().filter(|d| d.file_exported).count();
    println!(
        "刷新完成：共扫描 {} 个文件（{} 个导出），缓存已更新。",
        files_count, exported
    );
    Ok(())
}

/// `--find <ModuleName>`：按模块名列出所有导出 API。
fn cmd_find(
    module: &str,
    workspace: &Path,
    cache_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
) -> Result<()> {
    let docs = load_exported_docs(workspace, cache_dir, sources, config)?;
    let mut rows: Vec<(String, String)> = Vec::new();

    for doc in &docs {
        if !module_matches(&doc.module_name, module) {
            continue;
        }
        let rel = relative_path_from(workspace, &doc.file_path);
        for api in &doc.apis {
            if api.exported {
                rows.push((api.qualified_name(), rel.clone()));
            }
        }
    }

    if rows.is_empty() {
        println!("{} 未找到任何导出 API。", module_display(module));
    } else {
        println!("{} 共 {} 个导出 API：\n", module_display(module), rows.len());
        print_table_2col("API Name", "File", &rows);
    }
    Ok(())
}

/// 常规文档生成，可选 `--browse`。
fn cmd_generate(
    workspace: &Path,
    cache_dir: &Path,
    output_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
    browse: bool,
) -> Result<()> {
    let (all_docs, files_count) = collect_and_parse(workspace, cache_dir, sources, config)?;
    let exported_docs: Vec<_> = all_docs.into_iter().filter(|d| d.file_exported).collect();

    let readme_files = collect_readme_files(workspace)?;
    let generator = HtmlGenerator { theme: config.theme.clone() };
    generator.generate_site(&exported_docs, output_dir, workspace, &readme_files)?;

    println!(
        "ToDoc 完成：共处理 {} 个文件（导出 {} 个），输出目录 {}",
        files_count,
        exported_docs.len(),
        display_path(output_dir)
    );

    if browse {
        let index = output_dir.join("index.html");
        if index.exists() {
            open_in_browser(&index)?;
        } else {
            eprintln!("警告：输出目录中未找到 index.html");
        }
    }

    Ok(())
}

/// `--file <RelPath>`：仅对单个文件生成文档。
fn cmd_file(rel_path: &Path, workspace: &Path, _cache_dir: &Path, output_dir: &Path, config: &Config) -> Result<()> {
    let full_path = resolve_path(workspace, &rel_path.to_string_lossy());
    if !full_path.exists() {
        eprintln!("错误：文件不存在 — {}", full_path.display());
        std::process::exit(1);
    }

    let parser = LuaParser;
    let doc = parser.parse_file(&full_path, config)?;

    if !doc.file_exported {
        eprintln!(
            "警告：文件 {} 没有 ---<!export> 标记，跳过文档生成。",
            rel_path.display()
        );
        return Ok(());
    }

    let readme_files = collect_readme_files(workspace)?;
    let generator = HtmlGenerator { theme: config.theme.clone() };
    generator.generate_site(&[doc], output_dir, workspace, &readme_files)?;

    println!(
        "ToDoc 完成：已为 {} 生成文档，输出目录 {}",
        rel_path.display(),
        display_path(output_dir)
    );
    Ok(())
}

/// `--findapi` 统一入口：解析 flag 并分发。
fn cmd_findapi(
    args: &[String],
    workspace: &Path,
    cache_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
) -> Result<()> {
    let docs = load_exported_docs(workspace, cache_dir, sources, config)?;

    // 解析 flags: -r (regex), -f (function-only), -g (global module)。
    let mut use_regex = false;
    let mut func_only = false;
    let mut global_only = false;
    let mut positional: Vec<&str> = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-r" => use_regex = true,
            "-f" => func_only = true,
            "-g" => global_only = true,
            _ => positional.push(arg),
        }
    }

    if func_only {
        // --findapi -f <FuncName>
        if positional.is_empty() {
            eprintln!("错误：--findapi -f 需要提供函数名。");
            std::process::exit(1);
        }
        let pattern = positional[0];
        let matcher = build_name_matcher(pattern, use_regex)?;
        findapi_func_only(&docs, &matcher, workspace);
    } else if global_only {
        // --findapi -g <FuncName>  →  搜索 Global 模块。
        if positional.is_empty() {
            eprintln!("错误：--findapi -g 需要提供函数名。");
            std::process::exit(1);
        }
        let pattern = positional[0];
        let matcher = build_name_matcher(pattern, use_regex)?;
        findapi_module(&docs, "Global", &matcher, workspace);
    } else {
        // --findapi <ModuleName> <FuncName>
        if positional.len() < 2 {
            eprintln!("错误：--findapi 需要 <ModuleName> <FuncName>（或使用 -g 搜索全局模块，-f 跨模块搜索）。");
            std::process::exit(1);
        }
        let module = positional[0];
        let pattern = positional[1];
        let matcher = build_name_matcher(pattern, use_regex)?;
        findapi_module(&docs, module, &matcher, workspace);
    }

    Ok(())
}

/// 按模块+函数名搜索 API（输出 2 列）。
fn findapi_module(docs: &[FileDoc], module: &str, matcher: &NameMatcher, workspace: &Path) {
    let mut rows: Vec<(String, String)> = Vec::new();

    for doc in docs {
        if !module_matches(&doc.module_name, module) {
            continue;
        }
        let rel = relative_path_from(workspace, &doc.file_path);
        for api in &doc.apis {
            if api.exported && matcher.is_match(&api.name) {
                rows.push((api.qualified_name(), rel.clone()));
            }
        }
    }

    if rows.is_empty() {
        println!("未找到匹配的 API。");
    } else {
        println!("共找到 {} 个匹配 API：\n", rows.len());
        print_table_2col("API Name", "File", &rows);
    }
}

/// 仅按函数名跨模块搜索 API（输出 3 列）。
fn findapi_func_only(docs: &[FileDoc], matcher: &NameMatcher, workspace: &Path) {
    let mut rows: Vec<(String, String, String)> = Vec::new();

    for doc in docs {
        let rel = relative_path_from(workspace, &doc.file_path);
        for api in &doc.apis {
            if api.exported && matcher.is_match(&api.name) {
                rows.push((
                    api.qualified_name(),
                    doc.module_name.clone(),
                    rel.clone(),
                ));
            }
        }
    }

    if rows.is_empty() {
        println!("未找到匹配的 API。");
    } else {
        println!("共找到 {} 个匹配 API：\n", rows.len());
        print_table_3col("API Name", "Module", "File", &rows);
    }
}

// ─── 模块名匹配 ─────────────────────────────────────────────────────────────

/// 去除 `${...}` 包裹，返回内部名称。例如 `${Shop}` → `Shop`。
fn normalize_module_name(name: &str) -> &str {
    if let Some(inner) = name.strip_prefix("${") {
        inner.strip_suffix('}').unwrap_or(name)
    } else {
        name
    }
}

/// 模块名匹配：忽略大小写，且会自动剥离 `${...}` 包裹进行比较。
/// 即 `Shop` 可匹配 `${Shop}`，反之亦然。
fn module_matches(doc_module: &str, query: &str) -> bool {
    normalize_module_name(doc_module)
        .eq_ignore_ascii_case(normalize_module_name(query))
}

/// 模块名显示文本：Global 显示为 "全局模块 (Global)"，其他显示为 `模块 "..."`。
fn module_display(module: &str) -> String {
    if normalize_module_name(module).eq_ignore_ascii_case("Global") {
        "全局模块 (Global)".to_string()
    } else {
        format!("模块 \"{}\"", module)
    }
}

// ─── 名称匹配器 ─────────────────────────────────────────────────────────────

/// 函数名匹配器：支持精确、通配符 `.*`、正则三种模式。
enum NameMatcher {
    Exact(String),
    Regex(regex::Regex),
}

impl NameMatcher {
    fn is_match(&self, name: &str) -> bool {
        match self {
            NameMatcher::Exact(s) => s.eq_ignore_ascii_case(name),
            NameMatcher::Regex(re) => re.is_match(name),
        }
    }
}

/// 根据模式字符串和 use_regex 标志构建匹配器。
/// - `use_regex = true`：直接作为正则（TS 风格）。
/// - 包含 `.*`：自动转为正则（其余部分转义）。
/// - 否则精确匹配。
fn build_name_matcher(pattern: &str, use_regex: bool) -> Result<NameMatcher> {
    if use_regex {
        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("无效正则表达式 \"{}\": {}", pattern, e))?;
        return Ok(NameMatcher::Regex(re));
    }

    if pattern.contains(".*") {
        // 将 `.*` 以外的部分转义，再拼回完整正则。
        let mut re_str = String::from("(?i)^");
        for (i, segment) in pattern.split(".*").enumerate() {
            if i > 0 {
                re_str.push_str(".*");
            }
            re_str.push_str(&regex::escape(segment));
        }
        re_str.push('$');
        let re = regex::Regex::new(&re_str)
            .map_err(|e| format!("通配符转换失败 \"{}\": {}", pattern, e))?;
        Ok(NameMatcher::Regex(re))
    } else {
        Ok(NameMatcher::Exact(pattern.to_string()))
    }
}

// ─── 表格输出 ────────────────────────────────────────────────────────────────

fn print_table_2col(h1: &str, h2: &str, rows: &[(String, String)]) {
    let w1 = rows.iter().map(|(a, _)| a.len()).max().unwrap_or(0).max(h1.len());
    let w2 = rows.iter().map(|(_, b)| b.len()).max().unwrap_or(0).max(h2.len());
    println!("  {:<w1$}  {:<w2$}", h1, h2);
    println!("  {:-<w1$}  {:-<w2$}", "", "");
    for (a, b) in rows {
        println!("  {:<w1$}  {:<w2$}", a, b);
    }
}

fn print_table_3col(h1: &str, h2: &str, h3: &str, rows: &[(String, String, String)]) {
    let w1 = rows.iter().map(|(a, _, _)| a.len()).max().unwrap_or(0).max(h1.len());
    let w2 = rows.iter().map(|(_, b, _)| b.len()).max().unwrap_or(0).max(h2.len());
    let w3 = rows.iter().map(|(_, _, c)| c.len()).max().unwrap_or(0).max(h3.len());
    println!("  {:<w1$}  {:<w2$}  {:<w3$}", h1, h2, h3);
    println!("  {:-<w1$}  {:-<w2$}  {:-<w3$}", "", "", "");
    for (a, b, c) in rows {
        println!("  {:<w1$}  {:<w2$}  {:<w3$}", a, b, c);
    }
}

// ─── 路径解析 ────────────────────────────────────────────────────────────────

/// 将 `sources` 中的目录路径解析为基于 workspace 的绝对路径。
fn resolve_sources(
    workspace: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
) -> std::collections::BTreeMap<String, Vec<String>> {
    sources
        .iter()
        .map(|(dir_str, files)| {
            let resolved = resolve_path(workspace, dir_str);
            (resolved.to_string_lossy().to_string(), files.clone())
        })
        .collect()
}

// ─── 共享管线 ────────────────────────────────────────────────────────────────

/// 加载并返回所有已导出的文档。
fn load_exported_docs(
    workspace: &Path,
    cache_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
) -> Result<Vec<FileDoc>> {
    let (all_docs, _) = collect_and_parse(workspace, cache_dir, sources, config)?;
    Ok(all_docs.into_iter().filter(|d| d.file_exported).collect())
}

/// 收集并解析所有源文件，返回 (文档列表, 源文件总数)。
/// 当 `sources` 非空时，优先使用 sources 清单；否则回退到 workspace 目录全量扫描。
fn collect_and_parse(
    workspace: &Path,
    cache_dir: &Path,
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    config: &Config,
) -> Result<(Vec<FileDoc>, usize)> {
    let parser = LuaParser;
    let mut cache = CacheManager::new(cache_dir)?;

    let files = if sources.is_empty() {
        collect_source_files(workspace, parser.file_extensions())?
    } else {
        collect_source_files_from_entries(sources, parser.file_extensions())?
    };
    let files_count = files.len();
    let mut all_docs = Vec::new();

    for file in &files {
        let mtime = file_mtime(file)?;
        if let Some(cached) = cache.get_if_fresh(file, mtime) {
            all_docs.push(cached);
            continue;
        }

        let mut doc = parser.parse_file(file, config)?;
        doc.last_modified = mtime;
        cache.update(doc.clone());
        all_docs.push(doc);
    }

    cache.save()?;
    Ok((all_docs, files_count))
}

/// 计算相对于源码根目录的路径。
fn relative_path_from(source_root: &Path, file_path: &str) -> String {
    let root = source_root
        .canonicalize()
        .unwrap_or_else(|_| source_root.to_path_buf());
    let root_str = root.to_string_lossy();
    let root_str = root_str.strip_prefix(r"\\?\").unwrap_or(&root_str);

    let fp = file_path.strip_prefix(r"\\?\").unwrap_or(file_path);

    if let Some(rel) = fp.strip_prefix(root_str) {
        let rel = rel.trim_start_matches(['/', '\\']);
        if rel.is_empty() {
            fp.to_string()
        } else {
            rel.to_string()
        }
    } else {
        fp.to_string()
    }
}

/// 路径显示：去除 Windows `\\?\` 前缀后输出。
fn display_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// 用系统默认浏览器打开文件。
fn open_in_browser(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

// ─── 文件收集 ────────────────────────────────────────────────────────────────

/// 根据 `sources` 清单收集文件。
/// - value 为空数组 → 递归扫描该目录。
/// - value 有文件名 → 仅收集指定文件。
fn collect_source_files_from_entries(
    sources: &std::collections::BTreeMap<String, Vec<String>>,
    exts: &[&str],
) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for (dir_str, files) in sources {
        let dir = Path::new(dir_str);
        if files.is_empty() {
            // 全量扫描该目录。
            collect_source_files_inner(dir, exts, &mut out)?;
        } else {
            // 只收集指定文件。
            for f in files {
                let path = dir.join(f);
                if path.exists() {
                    out.push(path);
                } else {
                    eprintln!("警告：sources 中指定的文件不存在 — {}", path.display());
                }
            }
        }
    }
    Ok(out)
}

/// 递归收集支持扩展名的源文件。
fn collect_source_files(dir: &Path, exts: &[&str]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_source_files_inner(dir, exts, &mut out)?;
    Ok(out)
}

fn collect_source_files_inner(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_files_inner(&path, exts, out)?;
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if exts.iter().any(|target| target.eq_ignore_ascii_case(ext)) {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// 递归收集 README.md 文件。
fn collect_readme_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_readme_inner(dir, &mut out)?;
    Ok(out)
}

fn collect_readme_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_readme_inner(&path, out)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some("README.md") {
            out.push(path);
        }
    }
    Ok(())
}

/// 获取文件 mtime（秒级时间戳）。
fn file_mtime(path: &Path) -> Result<u64> {
    let mtime = fs::metadata(path)?
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    Ok(mtime)
}
