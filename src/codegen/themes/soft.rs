//! 柔和主题 CSS。

/// 返回内嵌 CSS 文本。
pub fn css() -> &'static str {
    r#"
:root {
  --bg: #FAFAF8;
  --text: #3C3C3C;
  --title: #2C5F7C;
  --code-bg: #F0EDE6;
  --link: #4A90A4;
  --todo: #C84B31;
  --deprecated: #888888;
  --card-border: #E6E2DA;
}
body {
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  line-height: 1.6;
  margin: 0;
  padding: 24px;
}
a { color: var(--link); text-decoration: none; }
a:hover { text-decoration: underline; }
h1, h2, h3 { color: var(--title); }
.container { max-width: 1080px; margin: 0 auto; }
.card {
  border: 1px solid var(--card-border);
  border-radius: 10px;
  background: #FFFFFF;
  box-shadow: 0 1px 2px rgba(0,0,0,0.04);
  padding: 16px;
  margin-bottom: 16px;
}
code, pre {
  background: var(--code-bg);
  border-radius: 6px;
}
pre { padding: 12px; overflow-x: auto; }
.param-heading { color: #1B5E20; font-weight: 600; }
code.param-name { color: #66BB6A; font-weight: 600; }
.param-type { color: #2E7D32; font-weight: 500; }
code.param-default { color: #8D6E63; font-weight: 600; }
.todo { color: var(--todo); font-weight: 600; }
.todo-heading { color: var(--todo); }
.note-heading { color: #4a7c59; font-weight: 600; }
.note-text { color: #4a7c59; margin: 4px 0; }
.note-text ul, .note-text ol { margin: 4px 0 4px 20px; }
.deprecated { color: var(--deprecated); text-decoration: line-through; }
.meta { color: #777; font-size: 0.92em; }
/* 俧边栏布局 */
.page-layout {
  display: flex;
  align-items: flex-start;
  gap: 20px;
  max-width: 1300px;
  margin: 0 auto;
}
.page-layout .container {
  flex: 1;
  min-width: 0;
  max-width: none;
  margin: 0;
}
.todo-sidebar {
  width: 210px;
  flex-shrink: 0;
  position: sticky;
  top: 24px;
  max-height: calc(100vh - 48px);
  overflow-y: auto;
  background: #FFFFFF;
  border: 1px solid var(--card-border);
  border-radius: 10px;
  padding: 12px 14px;
  box-shadow: 0 1px 2px rgba(0,0,0,0.04);
  font-size: 0.88em;
}
.todo-sidebar h3 {
  color: var(--todo);
  margin: 0 0 8px 0;
  font-size: 1em;
  font-weight: 700;
}
.todo-sidebar ul { margin: 0; padding: 0; list-style: none; }
.todo-sidebar li { margin-bottom: 10px; }
.todo-sidebar a { color: var(--todo); display: block; text-decoration: none; }
.todo-sidebar a:hover .sidebar-todo-text { text-decoration: underline; }
.sidebar-api-name { font-weight: 600; font-size: 0.95em; display: block; color: #3C3C3C; }
.sidebar-todo-text { font-size: 0.88em; display: block; word-break: break-word; }
"#
}
