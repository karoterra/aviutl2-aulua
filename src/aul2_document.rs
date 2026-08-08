#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aul2Document {
    pub preamble: Vec<Aul2PreambleNode>,
    pub sections: Vec<Aul2Section>,
    pub has_trailing_newline: bool,
}

impl Aul2Document {
    pub(crate) fn find_section(&self, name: &str) -> Option<&Aul2Section> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub(crate) fn find_section_mut(&mut self, name: &str) -> Option<&mut Aul2Section> {
        self.sections
            .iter_mut()
            .find(|section| section.name == name)
    }

    pub(crate) fn push_section(&mut self, section: Aul2Section) {
        self.sections.push(section);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Aul2PreambleNode {
    Comment(Aul2Comment),
    Blank(Aul2Blank),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aul2Section {
    pub name: String,
    pub source_line: Option<usize>,
    pub body: Vec<Aul2SectionNode>,
}

impl Aul2Section {
    pub(crate) fn find_entry(&self, key: &str) -> Option<&Aul2Entry> {
        self.body.iter().find_map(|node| match node {
            Aul2SectionNode::Entry(entry) if entry.key == key => Some(entry),
            _ => None,
        })
    }

    pub(crate) fn find_entry_mut(&mut self, key: &str) -> Option<&mut Aul2Entry> {
        self.body.iter_mut().find_map(|node| match node {
            Aul2SectionNode::Entry(entry) if entry.key == key => Some(entry),
            _ => None,
        })
    }

    pub(crate) fn insert_entry_at_end(&mut self, entry: Aul2Entry) {
        let insertion_index = self
            .body
            .iter()
            .rposition(|node| !matches!(node, Aul2SectionNode::Blank(_)))
            .map_or(0, |index| index + 1);
        self.body
            .insert(insertion_index, Aul2SectionNode::Entry(entry));
    }

    pub(crate) fn remove_entry(&mut self, key: &str) -> Option<Aul2Entry> {
        let index = self
            .body
            .iter()
            .position(|node| matches!(node, Aul2SectionNode::Entry(entry) if entry.key == key))?;

        match self.body.remove(index) {
            Aul2SectionNode::Entry(entry) => Some(entry),
            _ => unreachable!("entryのindexを検索済みです"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Aul2SectionNode {
    Entry(Aul2Entry),
    Comment(Aul2Comment),
    Blank(Aul2Blank),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aul2Entry {
    pub key: String,
    pub value_text: String,
    pub source_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aul2Comment {
    pub raw_text: String,
    pub source_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aul2Blank {
    pub raw_text: String,
    pub source_line: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value_text: &str, source_line: Option<usize>) -> Aul2Entry {
        Aul2Entry {
            key: key.to_string(),
            value_text: value_text.to_string(),
            source_line,
        }
    }

    fn comment(raw_text: &str, source_line: Option<usize>) -> Aul2Comment {
        Aul2Comment {
            raw_text: raw_text.to_string(),
            source_line,
        }
    }

    fn blank(raw_text: &str, source_line: Option<usize>) -> Aul2Blank {
        Aul2Blank {
            raw_text: raw_text.to_string(),
            source_line,
        }
    }

    fn section(name: &str, body: Vec<Aul2SectionNode>) -> Aul2Section {
        Aul2Section {
            name: name.to_string(),
            source_line: None,
            body,
        }
    }

    #[test]
    fn model_preserves_nodes_lines_order_and_trailing_newline() {
        let document = Aul2Document {
            preamble: vec![
                Aul2PreambleNode::Comment(comment("; preamble", Some(1))),
                Aul2PreambleNode::Blank(blank("\t", Some(2))),
            ],
            sections: vec![
                Aul2Section {
                    name: "first".to_string(),
                    source_line: Some(3),
                    body: vec![
                        Aul2SectionNode::Entry(entry("key", "a=b", Some(4))),
                        Aul2SectionNode::Comment(comment("; body", Some(5))),
                        Aul2SectionNode::Blank(blank("  ", Some(6))),
                    ],
                },
                Aul2Section {
                    name: "generated".to_string(),
                    source_line: None,
                    body: vec![Aul2SectionNode::Entry(entry("new", "", None))],
                },
            ],
            has_trailing_newline: false,
        };

        assert_eq!(
            document.preamble,
            vec![
                Aul2PreambleNode::Comment(comment("; preamble", Some(1))),
                Aul2PreambleNode::Blank(blank("\t", Some(2))),
            ]
        );
        assert_eq!(
            document.sections[0].body,
            vec![
                Aul2SectionNode::Entry(entry("key", "a=b", Some(4))),
                Aul2SectionNode::Comment(comment("; body", Some(5))),
                Aul2SectionNode::Blank(blank("  ", Some(6))),
            ]
        );
        assert_eq!(document.sections[0].source_line, Some(3));
        assert_eq!(document.sections[1].source_line, None);
        assert!(!document.has_trailing_newline);
    }

    #[test]
    fn section_and_entry_lookup_use_exact_comparison_and_ignore_other_nodes() {
        let composed = "é";
        let decomposed = "e\u{301}";
        let mut document = Aul2Document {
            preamble: Vec::new(),
            sections: vec![
                section(
                    " Name ",
                    vec![
                        Aul2SectionNode::Comment(comment("; Key", None)),
                        Aul2SectionNode::Blank(blank("", None)),
                        Aul2SectionNode::Entry(entry("Key", "first", None)),
                        Aul2SectionNode::Entry(entry(composed, "composed", None)),
                    ],
                ),
                section(decomposed, Vec::new()),
            ],
            has_trailing_newline: true,
        };

        assert!(document.find_section(" Name ").is_some());
        assert!(document.find_section("Name").is_none());
        assert!(document.find_section(" name ").is_none());
        assert!(document.find_section(composed).is_none());
        assert!(document.find_section(decomposed).is_some());

        let section = document.find_section(" Name ").unwrap();
        assert_eq!(section.find_entry("Key").unwrap().value_text, "first");
        assert!(section.find_entry("key").is_none());
        assert!(section.find_entry(" Key").is_none());
        assert!(section.find_entry(decomposed).is_none());
        assert_eq!(section.find_entry(composed).unwrap().value_text, "composed");

        document
            .find_section_mut(" Name ")
            .unwrap()
            .find_entry_mut("Key")
            .unwrap()
            .value_text = "changed".to_string();
        assert_eq!(
            document
                .find_section(" Name ")
                .unwrap()
                .find_entry("Key")
                .unwrap()
                .value_text,
            "changed"
        );
    }

    #[test]
    fn inserts_entry_before_all_trailing_blank_nodes() {
        let mut section = section(
            "section",
            vec![
                Aul2SectionNode::Entry(entry("existing", "value", Some(2))),
                Aul2SectionNode::Blank(blank("", Some(3))),
                Aul2SectionNode::Blank(blank("  ", Some(4))),
            ],
        );
        let new_entry = entry("new", "", None);

        section.insert_entry_at_end(new_entry.clone());

        assert_eq!(
            section.body,
            vec![
                Aul2SectionNode::Entry(entry("existing", "value", Some(2))),
                Aul2SectionNode::Entry(new_entry),
                Aul2SectionNode::Blank(blank("", Some(3))),
                Aul2SectionNode::Blank(blank("  ", Some(4))),
            ]
        );
    }

    #[test]
    fn insertion_only_treats_the_final_consecutive_blanks_as_trailing() {
        let mut section = section(
            "section",
            vec![
                Aul2SectionNode::Entry(entry("existing", "value", None)),
                Aul2SectionNode::Blank(blank("", None)),
                Aul2SectionNode::Comment(comment("; comment", None)),
                Aul2SectionNode::Blank(blank("\t", None)),
            ],
        );
        let new_entry = entry("new", "", None);

        section.insert_entry_at_end(new_entry.clone());

        assert_eq!(
            section.body,
            vec![
                Aul2SectionNode::Entry(entry("existing", "value", None)),
                Aul2SectionNode::Blank(blank("", None)),
                Aul2SectionNode::Comment(comment("; comment", None)),
                Aul2SectionNode::Entry(new_entry),
                Aul2SectionNode::Blank(blank("\t", None)),
            ]
        );
    }

    #[test]
    fn insertion_appends_after_an_entry_or_comment_without_trailing_blank() {
        let mut entry_ended = section(
            "entry-ended",
            vec![Aul2SectionNode::Entry(entry("existing", "value", None))],
        );
        entry_ended.insert_entry_at_end(entry("new", "", None));
        assert_eq!(
            entry_ended.body.last(),
            Some(&Aul2SectionNode::Entry(entry("new", "", None)))
        );

        let mut comment_ended = section(
            "comment-ended",
            vec![Aul2SectionNode::Comment(comment("; comment", None))],
        );
        comment_ended.insert_entry_at_end(entry("new", "", None));
        assert_eq!(
            comment_ended.body,
            vec![
                Aul2SectionNode::Comment(comment("; comment", None)),
                Aul2SectionNode::Entry(entry("new", "", None)),
            ]
        );
    }

    #[test]
    fn removes_only_the_matching_entry_and_preserves_other_nodes() {
        let original_body = vec![
            Aul2SectionNode::Comment(comment("; stale comment", Some(2))),
            Aul2SectionNode::Entry(entry("stale", "translation", Some(3))),
            Aul2SectionNode::Blank(blank("", Some(4))),
        ];
        let mut section = Aul2Section {
            name: "section".to_string(),
            source_line: Some(1),
            body: original_body,
        };

        assert_eq!(
            section.remove_entry("stale"),
            Some(entry("stale", "translation", Some(3)))
        );
        assert_eq!(
            section.body,
            vec![
                Aul2SectionNode::Comment(comment("; stale comment", Some(2))),
                Aul2SectionNode::Blank(blank("", Some(4))),
            ]
        );

        let body_after_removal = section.body.clone();
        assert_eq!(section.remove_entry("missing"), None);
        assert_eq!(section.body, body_after_removal);
        assert_eq!(section.name, "section");
        assert_eq!(section.source_line, Some(1));
    }

    #[test]
    fn push_section_only_appends_the_section() {
        let mut document = Aul2Document {
            preamble: Vec::new(),
            sections: vec![section("first", Vec::new())],
            has_trailing_newline: true,
        };
        let appended = section("second", Vec::new());

        document.push_section(appended.clone());

        assert_eq!(
            document.sections,
            vec![section("first", Vec::new()), appended]
        );
        assert!(
            document
                .sections
                .iter()
                .all(|section| section.body.is_empty())
        );
    }
}
