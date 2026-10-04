//! `stet inbox`: what's waiting in each repo's `Inbox/`, and each repo's "Mail that
//! belongs here" section as data, so the nightly mail run reads a list instead of prose.

use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::contract::{InboxItem, ItemKind, MailRules, RepoInbox};
use crate::markdown;

/// What an Inbox keeps for itself: never an item waiting.
const OWN: [&str; 3] = ["README.md", ".gitignore", "_processed"];

pub const MAIL_HEADING: &str = "Mail that belongs here";

pub fn inbox(root: &Path) -> RepoInbox {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = root.join("Inbox");
    let mut problems = vec![];
    let items = match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let mut items: Vec<InboxItem> = entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if OWN.contains(&name.as_str()) || name.starts_with('.') {
                        return None;
                    }
                    let meta = e.metadata().ok()?;
                    let modified = meta
                        .modified()
                        .ok()
                        .and_then(|t| jiff::Timestamp::try_from(t).ok())
                        .map(crate::contract_time)
                        .unwrap_or_default();
                    Some(InboxItem {
                        path: e.path().display().to_string(),
                        kind: if meta.is_dir() {
                            ItemKind::Folder
                        } else {
                            ItemKind::File
                        },
                        bytes: size(&e.path()),
                        modified,
                        name,
                    })
                })
                .collect();
            items.sort_by(|a, b| a.name.cmp(&b.name));
            items
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(e) => {
            problems.push(format!("can't read Inbox/: {e}"));
            vec![]
        }
    };
    let mail = match std::fs::read_to_string(root.join("AGENTS.md")) {
        Ok(text) => {
            let mail = mail_rules(&text);
            if mail.is_none() {
                problems.push(format!("AGENTS.md has no \"{MAIL_HEADING}\" section"));
            }
            mail
        }
        Err(e) => {
            problems.push(format!("can't read AGENTS.md: {e}"));
            None
        }
    };
    RepoInbox {
        name,
        path: root.display().to_string(),
        inbox: dir.display().to_string(),
        items,
        mail,
        problem: (!problems.is_empty()).then(|| problems.join("; ")),
    }
}

fn size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| size(&e.path())).sum())
        .unwrap_or(0)
}

/// Parses the "Mail that belongs here" section: its bullets, its "Not here:" paragraph,
/// and any other paragraph after the bullets. The paragraph before them describes the
/// nightly run and is the same in every repo, so it's left out.
pub fn mail_rules(text: &str) -> Option<MailRules> {
    let mut rules = MailRules::default();
    let mut in_section: Option<HeadingLevel> = None;
    let mut heading: Option<(HeadingLevel, String)> = None;
    let mut found = false;
    let mut seen_list = false;
    let mut depth = 0usize;
    for (event, range) in Parser::new_ext(text, markdown::options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if in_section.is_some_and(|l| level <= l) {
                    break;
                }
                heading = Some((level, String::new()));
            }
            Event::Text(t) if heading.is_some() => {
                if let Some((_, h)) = heading.as_mut() {
                    h.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, h)) = heading.take()
                    && in_section.is_none()
                    && h.trim().eq_ignore_ascii_case(MAIL_HEADING)
                {
                    in_section = Some(level);
                    found = true;
                }
            }
            _ if in_section.is_none() => {}
            Event::Start(Tag::List(_)) => {
                depth += 1;
                seen_list = true;
            }
            Event::End(TagEnd::List(_)) => depth = depth.saturating_sub(1),
            Event::Start(Tag::Item) if depth == 1 => {
                let item = text[range].trim_start();
                let item = item
                    .strip_prefix(['-', '*', '+'])
                    .unwrap_or(item)
                    .trim_start();
                rules.belongs.push(one_line(item));
            }
            Event::Start(Tag::Paragraph) if depth == 0 && seen_list => {
                let para = one_line(&text[range]);
                match para.strip_prefix("Not here:") {
                    Some(rest) => rules.not_here = Some(rest.trim().to_string()),
                    None => rules.notes.push(para),
                }
            }
            _ => {}
        }
    }
    found.then_some(rules)
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENTS: &str = "# Repo\n\n## Mail that belongs here\n\nThe nightly run reads this.\n\n\
        - Travel bookings, tickets, and\n  event listings.\n- Mostly absorbed.\n  - nested\n\n\
        Not here: Health, Finances.\n\nA question email names only the sender.\n\n\
        ### Detail\n\nStill in.\n\n## Next\n\n- not mail\n";

    #[test]
    fn parses_the_section() {
        let rules = mail_rules(AGENTS).unwrap();
        assert_eq!(
            rules.belongs,
            [
                "Travel bookings, tickets, and event listings.",
                "Mostly absorbed. - nested"
            ]
        );
        assert_eq!(rules.not_here.as_deref(), Some("Health, Finances."));
        assert_eq!(
            rules.notes,
            ["A question email names only the sender.", "Still in."]
        );
    }

    #[test]
    fn no_section_is_none() {
        assert!(mail_rules("# Repo\n\n## Other\n\n- x\n").is_none());
    }
}
