use mdbook_preprocessor::book::{Book, Chapter};
use mdbook_preprocessor::errors::Result;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use std::io;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("supports") => {
            let renderer = args.next().expect("Renderer not specified");
            handle_supports(&renderer);
        }
        Some(arg) => {
            eprintln!("Unknown argument: {}", arg);
            std::process::exit(1);
        }
        None => {}
    }

    if let Err(e) = handle_preprocess() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn handle_supports(renderer: &str) {
    let preprocessor = CodeblockTitlePreprocessor;
    let supported = preprocessor.supports_renderer(&renderer).unwrap_or(false);
    std::process::exit(if supported { 0 } else { 1 });
}

fn handle_preprocess() -> Result<()> {
    let preprocessor = CodeblockTitlePreprocessor;
    let (ctx, book) = mdbook_preprocessor::parse_input(std::io::stdin())?;

    let processed_book = preprocessor.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

struct CodeblockTitlePreprocessor;

impl Preprocessor for CodeblockTitlePreprocessor {
    fn name(&self) -> &str {
        "codeblock-title"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        book.for_each_chapter_mut(transform_chapter);
        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        Ok(renderer == "html" || renderer == "markdown")
    }
}

fn transform_chapter(chapter: &mut Chapter) {
    chapter.content = transform_markdown(&chapter.content);
}

fn transform_markdown(input: &str) -> String {
    let mut lines = Vec::<String>::new();
    let mut in_titled_block = false;

    for line in input.lines() {
        if !in_titled_block {
            if let Some(parsed) = parse_codeblock_title_line(line) {
                lines.push(r#"<div class="codeblock-with-title">"#.to_string());
                lines.push(format!(
                    r#"<div class="codeblock-title">{}</div>"#,
                    escape_html(&parsed.filename)
                ));
                lines.push("".to_string());
                lines.push(format!("```{}", parsed.language));
                in_titled_block = true;
                continue;
            }
        } else if line.trim() == "```" {
            lines.push("```".to_string());
            lines.push("".to_string());
            lines.push("</div>".to_string());
            in_titled_block = false;
            continue;
        }

        lines.push(line.to_string());
    }

    lines.push("".to_string());
    lines.join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeblockTitle {
    language: String,
    filename: String,
}

fn parse_codeblock_title_line(line: &str) -> Option<CodeblockTitle> {
    let rest = line.strip_prefix("```")?;
    let (language, filename) = rest.split_once(':')?;

    if language.is_empty() || filename.is_empty() {
        return None;
    }

    Some(CodeblockTitle {
        language: language.to_string(),
        filename: filename.to_string(),
    })
}

fn escape_html(text: &str) -> String {
    let mut escaped_text = String::new();

    for ch in text.chars() {
        match ch {
            '&' => escaped_text.push_str("&amp;"),
            '<' => escaped_text.push_str("&lt;"),
            '>' => escaped_text.push_str("&gt;"),
            '"' => escaped_text.push_str("&quot;"),
            '\'' => escaped_text.push_str("&apos;"),
            _ => escaped_text.push(ch),
        }
    }

    escaped_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_titled_codeblock() {
        let parsed = parse_codeblock_title_line("```rust:src/main.rs").unwrap();
        assert_eq!(parsed.language, "rust");
        assert_eq!(parsed.filename, "src/main.rs");
    }

    #[test]
    fn parse_normal_codeblock_returns_none() {
        assert_eq!(parse_codeblock_title_line("```rust"), None);
        assert_eq!(parse_codeblock_title_line("```"), None);
    }

    #[test]
    fn transform_titled_codeblock() {
        let input = r#"
```rust:src/main.rs
fn main() {}
```
"#;
        let expected = r#"
<div class="codeblock-with-title">
<div class="codeblock-title">src/main.rs</div>

```rust
fn main() {}
```

</div>
"#;
        let output = transform_markdown(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn transform_normal_codeblock_is_unchanged() {
        let input = r#"
```rust
fn main() {}
```
"#;
        let output = transform_markdown(input);
        assert_eq!(output, input);
    }
}
