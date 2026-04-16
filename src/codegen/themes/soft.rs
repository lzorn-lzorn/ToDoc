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
.todo { color: var(--todo); font-weight: 600; }
.deprecated { color: var(--deprecated); text-decoration: line-through; }
.meta { color: #777; font-size: 0.92em; }
"#
}
