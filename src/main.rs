//! ToDoc CLI 入口。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::Parser;

use todoc::cache::CacheManager;
use todoc::codegen::html_generator::HtmlGenerator;
use todoc::config::Config;
use todoc::lang::lua::LuaParser;
use todoc::lang::LanguageParser;
use todoc::Result;

/// 命令行参数。
#[derive(Debug, Parser)]
#[command(name = "todoc", version, about = "从源码注释生成 API HTML 文档")]
struct Cli {
    /// 要扫描的源码目录。
    #[arg(long)]
    dir: PathBuf,
    /// 输出目录（覆盖配置）。
    #[arg(long)]
    out: Option<PathBuf>,
    /// 配置文件路径。
    #[arg(long, default_value = "todoc.json")]
    config: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load(&cli.config)?;
    if let Some(out) = &cli.out {
        config.output_dir = out.to_string_lossy().to_string();
    }

    let parser = LuaParser;
    let mut cache = CacheManager::new(Path::new(&config.cache_dir))?;

    let files = collect_source_files(&cli.dir, parser.file_extensions())?;
    let mut docs = Vec::new();

    for file in files {
        let mtime = file_mtime(&file)?;
        if let Some(cached) = cache.get_if_fresh(&file, mtime) {
            docs.push(cached);
            continue;
        }

        let mut doc = parser.parse_file(&file, &config)?;
        doc.last_modified = mtime;
        cache.update(doc.clone());
        docs.push(doc);
    }

    let generator = HtmlGenerator;
    generator.generate_site(&docs, Path::new(&config.output_dir))?;
    cache.save()?;

    println!(
        "ToDoc 完成：共处理 {} 个文件，输出目录 {}",
        docs.len(),
        config.output_dir
    );

    Ok(())
}

/// 递归收集支持扩展名的源文件。
fn collect_source_files(dir: &Path, exts: &[&str]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_source_files_inner(dir, exts, &mut out)?;
    Ok(out)
}

/// 递归遍历目录。
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

/// 获取文件 mtime（秒级时间戳）。
fn file_mtime(path: &Path) -> Result<u64> {
    let mtime = fs::metadata(path)?
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    Ok(mtime)
}
