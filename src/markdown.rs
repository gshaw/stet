//! Reading Markdown for the house rules: which files, their blocks of text, their links
//! and the anchors their headings make.

use std::collections::HashSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// gitignore-style patterns, matched against paths relative to `root`.
pub struct Patterns {
    matcher: Gitignore,
}

impl Patterns {
    pub fn new(root: &Path, patterns: impl IntoIterator<Item = String>) -> Patterns {
        let mut builder = GitignoreBuilder::new(root);
        for pattern in patterns {
            // A bad pattern is reported by the tools that read the same config.
            let _ = builder.add_line(None, &pattern);
        }
        Patterns {
            matcher: builder.build().unwrap_or_else(|_| Gitignore::empty()),
        }
    }

    /// True when `rel` (relative to the root) or a folder above it matches.
    pub fn matches(&self, rel: &Path) -> bool {
        self.matcher
            .matched_path_or_any_parents(rel, false)
            .is_ignore()
    }
}

/// Every Markdown file under `root` the house rules read: hidden folders included,
/// `.gitignore`d files and `skip` left out. Sorted, relative to the root.
pub fn files(root: &Path, skip: &Patterns) -> Vec<PathBuf> {
    let mut out = vec![];
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .parents(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if !entry.file_type().is_some_and(|t| t.is_file())
            || path.extension().is_none_or(|e| e != "md")
        {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        if !skip.matches(rel) {
            out.push(rel.to_path_buf());
        }
    }
    out.sort();
    out
}

/// Byte offset to 1-based line number.
pub struct Lines {
    starts: Vec<usize>,
}

impl Lines {
    pub fn new(text: &str) -> Lines {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Lines { starts }
    }

    pub fn line(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }
}

pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
}

/// The line count of a YAML frontmatter block at the top, if there is one.
pub fn frontmatter(text: &str) -> Option<usize> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    lines
        .position(|l| matches!(l.trim_end(), "---" | "..."))
        .map(|i| i + 2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Heading,
    Paragraph,
    Item,
    Cell,
}

/// A run of prose: a paragraph, heading, list item's own text, or table cell.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub start: usize,
    /// The text without inline code.
    pub prose: String,
    /// The text with inline code.
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub image: bool,
    /// The whole link in the source.
    pub range: Range<usize>,
    /// Its text in the source, between the brackets.
    pub inner: Option<Range<usize>>,
    pub autolink: bool,
}

pub struct Doc {
    pub blocks: Vec<Block>,
    pub links: Vec<Link>,
}

pub fn parse(text: &str) -> Doc {
    let mut blocks: Vec<Block> = vec![];
    let mut stack: Vec<Block> = vec![];
    let mut links: Vec<Link> = vec![];
    let mut open_links: Vec<Link> = vec![];
    let mut code_block = false;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        for link in open_links.iter_mut() {
            if !matches!(event, Event::End(TagEnd::Link | TagEnd::Image)) {
                let inner = link.inner.get_or_insert(range.clone());
                inner.start = inner.start.min(range.start);
                inner.end = inner.end.max(range.end);
            }
        }
        match event {
            Event::Start(tag) => {
                let kind = match &tag {
                    Tag::Heading { .. } => Some(BlockKind::Heading),
                    Tag::Paragraph => Some(BlockKind::Paragraph),
                    Tag::Item => Some(BlockKind::Item),
                    Tag::TableCell => Some(BlockKind::Cell),
                    _ => None,
                };
                if let Some(kind) = kind {
                    stack.push(Block {
                        kind,
                        start: range.start,
                        prose: String::new(),
                        text: String::new(),
                    });
                }
                match tag {
                    Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::MetadataBlock(_) => code_block = true,
                    Tag::Link {
                        dest_url,
                        link_type,
                        ..
                    } => open_links.push(Link {
                        url: dest_url.to_string(),
                        image: false,
                        range: range.clone(),
                        inner: None,
                        autolink: matches!(
                            link_type,
                            pulldown_cmark::LinkType::Autolink | pulldown_cmark::LinkType::Email
                        ),
                    }),
                    Tag::Image { dest_url, .. } => open_links.push(Link {
                        url: dest_url.to_string(),
                        image: true,
                        range: range.clone(),
                        inner: None,
                        autolink: false,
                    }),
                    _ => {}
                }
            }
            Event::End(end) => match end {
                TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::Item | TagEnd::TableCell => {
                    if let Some(block) = stack.pop()
                        && !block.text.trim().is_empty()
                    {
                        blocks.push(block);
                    }
                }
                TagEnd::CodeBlock | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {
                    code_block = false
                }
                TagEnd::Link | TagEnd::Image => {
                    if let Some(link) = open_links.pop() {
                        links.push(link);
                    }
                }
                _ => {}
            },
            Event::Text(t) if !code_block => {
                if let Some(top) = stack.last_mut() {
                    top.prose.push_str(&t);
                    top.text.push_str(&t);
                }
            }
            Event::Code(t) => {
                if let Some(top) = stack.last_mut() {
                    top.prose.push(' ');
                    top.text.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(top) = stack.last_mut() {
                    top.prose.push(' ');
                    top.text.push(' ');
                }
            }
            _ => {}
        }
    }
    blocks.sort_by_key(|b| b.start);
    links.sort_by_key(|l| l.range.start);
    Doc { blocks, links }
}

/// Every anchor a file's headings make on GitHub, plus explicit `id`/`name` anchors.
pub fn anchors(text: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut heading: Option<String> = None;
    for event in Parser::new_ext(text, options()) {
        match event {
            Event::Start(Tag::Heading { id, .. }) => {
                if let Some(id) = id {
                    seen.insert(id.to_lowercase());
                }
                heading = Some(String::new());
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = heading.take() {
                    let base = slug(&h);
                    let mut candidate = base.clone();
                    let mut n = 0;
                    while seen.contains(&candidate) {
                        n += 1;
                        candidate = format!("{base}-{n}");
                    }
                    seen.insert(candidate);
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = heading.as_mut() {
                    h.push_str(&t);
                }
            }
            Event::Html(h) | Event::InlineHtml(h) => {
                for id in html_ids(&h) {
                    seen.insert(id.to_lowercase());
                }
            }
            _ => {}
        }
    }
    seen
}

fn html_ids(html: &str) -> Vec<String> {
    let mut out = vec![];
    for key in ["id=\"", "name=\""] {
        let mut rest = html;
        while let Some(at) = rest.find(key) {
            rest = &rest[at + key.len()..];
            if let Some(end) = rest.find('"') {
                out.push(rest[..end].to_string());
            }
        }
    }
    out
}

/// GitHub's heading slug: lowercase, punctuation and symbols dropped, spaces to hyphens.
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_github() {
        assert_eq!(
            slug("✅ Superseded 2026-08-14 — the decode happened, fixed code won"),
            "-superseded-2026-08-14--the-decode-happened-fixed-code-won"
        );
        assert_eq!(slug("A family name"), "a-family-name");
        assert_eq!(slug("`stet check` & co."), "stet-check--co");
        assert_eq!(slug("Open questions ⚠️"), "open-questions-");
    }

    #[test]
    fn duplicate_headings_count_up() {
        let a = anchors("# One\n\n## One\n\n### One\n");
        assert!(a.contains("one") && a.contains("one-1") && a.contains("one-2"));
    }

    #[test]
    fn frontmatter_needs_a_close() {
        assert_eq!(frontmatter("---\ntitle: x\n---\nbody\n"), Some(3));
        assert_eq!(frontmatter("---\nno close\n"), None);
        assert_eq!(frontmatter("# Title\n---\n"), None);
    }

    #[test]
    fn blocks_skip_code_and_keep_items() {
        let doc = parse("# Head\n\n- item `code` one\n  - nested\n\n```\ncurrently\n```\n");
        let texts: Vec<_> = doc
            .blocks
            .iter()
            .map(|b| b.prose.trim().to_string())
            .collect();
        assert_eq!(texts, ["Head", "item   one", "nested"]);
    }

    #[test]
    fn links_know_their_text() {
        let text = "See [the note](a.md#x) and ![img](p.png).\n";
        let doc = parse(text);
        assert_eq!(doc.links.len(), 2);
        let link = &doc.links[0];
        assert_eq!(&text[link.range.clone()], "[the note](a.md#x)");
        assert_eq!(&text[link.inner.clone().unwrap()], "the note");
    }

    #[test]
    fn lines_are_one_based() {
        let lines = Lines::new("a\nb\nc");
        assert_eq!(lines.line(0), 1);
        assert_eq!(lines.line(2), 2);
        assert_eq!(lines.line(4), 3);
    }
}
