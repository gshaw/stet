//! `stet help <topic>`: the reference, built into the binary from `Docs/help/*.md` so it
//! always matches the code it describes.

pub const TOPICS: [(&str, &str, &str); 2] = [
    (
        "config",
        ".stet.toml: ignores, frontmatter paths, check tasks",
        include_str!("../Docs/help/config.md"),
    ),
    (
        "rules",
        "the house rules stet check applies",
        include_str!("../Docs/help/rules.md"),
    ),
];

pub fn topic(name: &str) -> Option<&'static str> {
    TOPICS.iter().find(|(n, _, _)| *n == name).map(|t| t.2)
}
