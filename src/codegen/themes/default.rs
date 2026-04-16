//! 默认主题 CSS：Gruvbox 代码块 + 参数名浅绿 + "参数"深绿。

/// 返回内嵌 CSS 文本。
pub fn css() -> &'static str {
    r#"
:root {
  --bg: #FAFAF8;
  --text: #3C3C3C;
  --title: #2C5F7C;
  --link: #4A90A4;
  --todo: #C84B31;
  --deprecated: #888888;
  --card-border: #E6E2DA;
  /* Gruvbox 代码配色 */
  --code-bg: #282828;
  --code-fg: #ebdbb2;
  --code-border: #3c3836;
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

/* ── Gruvbox 代码块（行内 + 块级） ── */
code {
  background: var(--code-bg);
  color: var(--code-fg);
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 0.92em;
}
pre {
  background: var(--code-bg);
  color: var(--code-fg);
  border: 1px solid var(--code-border);
  border-radius: 6px;
  padding: 12px;
  overflow-x: auto;
}
pre code {
  background: none;
  padding: 0;
  border-radius: 0;
}

/* ── 参数样式 ── */
.param-heading { color: #1B5E20; font-weight: 600; }
.param-name { color: #66BB6A; font-weight: 600; }

/* ── 其他 ── */
.todo { color: var(--todo); font-weight: 600; }
.todo-heading { color: var(--todo); }
.note-text { color: #4a7c59; margin: 4px 0; }
.deprecated { color: var(--deprecated); text-decoration: line-through; }
.meta { color: #777; font-size: 0.92em; }
"#
}
