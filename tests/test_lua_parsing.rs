//! Lua 解析能力测试。

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use todoc::config::Config;
use todoc::lang::lua::LuaParser;
use todoc::lang::LanguageParser;

/// 创建唯一临时 Lua 文件路径。
fn temp_lua_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("todoc_test_{}.lua", ts))
}

#[test]
fn test_parse_lua_file() {
    // 准备测试文件。
    let path = temp_lua_path();
    let content = r#"
--- 文件概述第一行
--- 文件概述第二行

require("alpha.core")

---@brief: 求和函数
---@param \name{x} \type{number} \content{输入 x}
---@param \name{y} \type{number} y 的描述
---@return \type{number} \content{返回结果}
---@todo \content{补充边界测试}
---@export
local function add(x, y)
  return x + y
end

---@brief 表方法
function M:ping(name)
  return name
end
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    // 执行解析。
    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    // 验证文件级信息。
    assert!(doc.overview.contains("文件概述第一行"));
    assert_eq!(doc.dependencies, vec!["alpha.core"]);
    assert_eq!(doc.apis.len(), 2);

    // 验证第一个函数。
    let first = &doc.apis[0];
    assert_eq!(first.name, "add");
    assert!(first.exported); // 显式 @export
    assert_eq!(first.signature, "local function add(x, y)");
    assert_eq!(first.params.len(), 2);
    assert_eq!(first.returns.len(), 1);
    assert_eq!(first.todos.len(), 1);

    // 验证表方法识别。
    let second = &doc.apis[1];
    assert_eq!(second.name, "ping");
    assert_eq!(second.table_name.as_deref(), Some("M"));
    assert_eq!(second.signature, "function M:ping(name)");
    assert!(second.exported); // 表方法默认导出

    // 清理临时文件。
    let _ = fs::remove_file(path);
}

#[test]
fn test_comment_prefix_before_first_tag_and_breif_alias() {
    let path = temp_lua_path();
    let content = r#"
---@! 这是警示前缀，不应并入 TODO
---@! 第二行前缀
---@todo: 只应该包含这一行
---@breif: 这是简述
local function sample()
  return true
end
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    assert_eq!(doc.apis.len(), 1);
    let api = &doc.apis[0];

    assert_eq!(api.todos.len(), 1);
    assert_eq!(api.todos[0], "只应该包含这一行");
    assert_eq!(api.brief.as_deref(), Some("这是简述"));
    // local function 没有 @export，默认不导出。
    assert!(!api.exported);

    let _ = fs::remove_file(path);
}

#[test]
fn test_inner_functions_skipped() {
    let path = temp_lua_path();
    let content = r#"
---@brief 外部函数
function Outer.foo(x)
    local function inner_helper(y)
        return y + 1
    end
    return inner_helper(x)
end

---@brief 第二个
function Outer.bar()
end
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    // inner_helper 不应被收集。
    assert_eq!(doc.apis.len(), 2);
    assert_eq!(doc.apis[0].name, "foo");
    assert_eq!(doc.apis[0].signature, "function Outer.foo(x)");
    assert_eq!(doc.apis[1].name, "bar");

    let _ = fs::remove_file(path);
}

#[test]
fn test_no_tag_table_method_not_exported() {
    let path = temp_lua_path();
    let content = r#"
---@brief 有注释的函数
function T.documented()
end

function T.undocumented()
end

-- 只有纯文本描述，没有标签
function T.plain_comment()
end
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    assert_eq!(doc.apis.len(), 3);
    // 有 @brief 标签 → 导出。
    assert!(doc.apis[0].exported);
    assert_eq!(doc.apis[0].name, "documented");
    // 无注释、无标签 → 不导出。
    assert!(!doc.apis[1].exported);
    assert_eq!(doc.apis[1].name, "undocumented");
    // 有纯文本注释但无 @tag → 不导出。
    assert!(!doc.apis[2].exported);
    assert_eq!(doc.apis[2].name, "plain_comment");

    let _ = fs::remove_file(path);
}

#[test]
fn test_overview_skips_blank_lines() {
    let path = temp_lua_path();
    let content = r#"-- 概述第一行

-- 概述第三行（上面有空行）

-- 概述第五行

local x = 1
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    assert!(doc.overview.contains("概述第一行"));
    assert!(doc.overview.contains("概述第三行"));
    assert!(doc.overview.contains("概述第五行"));

    let _ = fs::remove_file(path);
}

#[test]
fn test_block_comment_support() {
    let path = temp_lua_path();
    let content = r#"
-- @brief 价格信息
-- @param iStoreId 商店id
--[[
{
    iCoinItemId = int,
    iNeedCoinNum = int,
}
]]
function T.GetPrice(iStoreId)
end

--[[ 单行块注释 ]]
function T.Inline(x)
end
"#;
    fs::write(&path, content).expect("应能写入临时 Lua 文件");

    let parser = LuaParser;
    let doc = parser
        .parse_file(&path, &Config::default())
        .expect("Lua 文件应能成功解析");

    // 第一个函数：有 @brief 和 @param 标签 + 块注释内容。
    assert_eq!(doc.apis[0].name, "GetPrice");
    assert!(doc.apis[0].exported);
    assert!(doc.apis[0].brief.as_deref().unwrap().contains("价格信息"));
    // 块注释内容应被收入 raw_comment。
    assert!(doc.apis[0].raw_comment.contains("iCoinItemId"));

    // 第二个函数：有单行块注释（纯文本，无标签） → 不导出。
    assert_eq!(doc.apis[1].name, "Inline");
    assert!(!doc.apis[1].exported);

    let _ = fs::remove_file(path);
}

#[test]
fn test_file_export_marker() {
    // 有 ---<!export> 标记的文件。
    let path1 = temp_lua_path();
    let content1 = "---<!export>\n-- 概述\nfunction T.foo()\nend\n";
    fs::write(&path1, content1).expect("写入");
    let doc1 = LuaParser.parse_file(&path1, &Config::default()).unwrap();
    assert!(doc1.file_exported);
    // 导出标记不应出现在 overview 中。
    assert!(!doc1.overview.contains("<!export>"));

    // 有前导空行再跟 ---<!export>。
    let path2 = temp_lua_path();
    let content2 = "\n\n---<!export>\n-- 概述\nfunction T.bar()\nend\n";
    fs::write(&path2, content2).expect("写入");
    let doc2 = LuaParser.parse_file(&path2, &Config::default()).unwrap();
    assert!(doc2.file_exported);

    // 没有标记的文件。
    let path3 = temp_lua_path();
    let content3 = "-- 普通文件\nfunction T.baz()\nend\n";
    fs::write(&path3, content3).expect("写入");
    let doc3 = LuaParser.parse_file(&path3, &Config::default()).unwrap();
    assert!(!doc3.file_exported);

    let _ = fs::remove_file(path1);
    let _ = fs::remove_file(path2);
    let _ = fs::remove_file(path3);
}

#[test]
fn test_export_marker_with_module_name() {
    // 带模块名的导出标记。
    let path1 = temp_lua_path();
    let content1 = "---<!export ShopHelper>\n-- 商店模块\nfunction T.foo()\nend\n";
    fs::write(&path1, content1).expect("写入");
    let doc1 = LuaParser.parse_file(&path1, &Config::default()).unwrap();
    assert!(doc1.file_exported);
    assert_eq!(doc1.module_name, "ShopHelper");
    assert!(!doc1.overview.contains("<!export"));

    // 无模块名默认 "Global"。
    let path2 = temp_lua_path();
    let content2 = "---<!export>\n-- 概述\nfunction T.bar()\nend\n";
    fs::write(&path2, content2).expect("写入");
    let doc2 = LuaParser.parse_file(&path2, &Config::default()).unwrap();
    assert!(doc2.file_exported);
    assert_eq!(doc2.module_name, "Global");

    // 无标记也应默认 "Global"。
    let path3 = temp_lua_path();
    let content3 = "-- 普通文件\nfunction T.baz()\nend\n";
    fs::write(&path3, content3).expect("写入");
    let doc3 = LuaParser.parse_file(&path3, &Config::default()).unwrap();
    assert!(!doc3.file_exported);
    assert_eq!(doc3.module_name, "Global");

    let _ = fs::remove_file(path1);
    let _ = fs::remove_file(path2);
    let _ = fs::remove_file(path3);
}

#[test]
fn test_continuation_lines_joined() {
    let path = temp_lua_path();
    // 测试 `@ + 空白` 延续行被合并到前一行。
    let content = "\
--- @breif 通过 iStoreId 获取价格信息, 注意! 这个接口只会默认拿第一个, 要拿到完整\n\
--- @      信息, 使用 ShopManager 的接口\n\
--- @param bGetFirst, 默认获取第一个货币价格信息\n\
function T.GetPriceInfo(iStoreId, bGetFirst)\n\
end\n";
    fs::write(&path, content).expect("写入");

    let doc = LuaParser
        .parse_file(&path, &Config::default())
        .unwrap();

    assert_eq!(doc.apis.len(), 1);
    let api = &doc.apis[0];
    // brief 应该将延续行合并，不含多余的 @。
    let brief = api.brief.as_deref().unwrap();
    assert!(
        brief.contains("完整 信息"),
        "brief 应合并延续行: {:?}",
        brief
    );
    assert!(
        !brief.contains('@'),
        "brief 不应包含单独的 @: {:?}",
        brief
    );
    // param 应正常解析。
    assert_eq!(api.params.len(), 1);
    assert_eq!(api.params[0].name, "bGetFirst,");

    let _ = fs::remove_file(path);
}
