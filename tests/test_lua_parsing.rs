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
    assert!(first.exported);
    assert_eq!(first.params.len(), 2);
    assert_eq!(first.returns.len(), 1);
    assert_eq!(first.todos.len(), 1);

    // 验证表方法识别。
    let second = &doc.apis[1];
    assert_eq!(second.name, "ping");
    assert_eq!(second.table_name.as_deref(), Some("M"));

    // 清理临时文件。
    let _ = fs::remove_file(path);
}
