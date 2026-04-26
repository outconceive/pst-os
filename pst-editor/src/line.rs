use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::styles::{block, inline};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockState {
    None,
    CodeBlockStart,
    CodeBlockBody,
    CodeBlockEnd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaLine {
    pub format: char,
    pub level: u8,
    pub state: BlockState,
}

impl MetaLine {
    pub fn new() -> Self {
        Self {
            format: block::PLAIN,
            level: 0,
            state: BlockState::None,
        }
    }

    pub fn with_format(format: char, level: u8) -> Self {
        Self { format, level, state: BlockState::None }
    }

    pub fn is_heading(&self) -> bool {
        self.format == block::HEADING
    }

    pub fn is_list(&self) -> bool {
        self.format == block::LIST_UNORDERED || self.format == block::LIST_ORDERED
    }

    pub fn is_code_block(&self) -> bool {
        self.format == block::CODE_BLOCK
            || matches!(
                self.state,
                BlockState::CodeBlockStart | BlockState::CodeBlockBody | BlockState::CodeBlockEnd
            )
    }
}

impl Default for MetaLine {
    fn default() -> Self { Self::new() }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub meta: MetaLine,
    pub content: String,
    pub styles: String,
    pub links: Option<BTreeMap<usize, String>>,
}

impl Line {
    pub fn new() -> Self {
        Self {
            meta: MetaLine::new(),
            content: String::new(),
            styles: String::new(),
            links: None,
        }
    }

    pub fn with_content(content: &str) -> Self {
        let len = content.len();
        Self {
            meta: MetaLine::new(),
            content: String::from(content),
            styles: core::iter::repeat(inline::PLAIN).take(len).collect(),
            links: None,
        }
    }

    pub fn len(&self) -> usize { self.content.len() }

    pub fn is_empty(&self) -> bool { self.content.is_empty() }

    pub fn insert_char(&mut self, col: usize, ch: char, style: char) {
        let col = col.min(self.content.len());
        self.content.insert(col, ch);
        self.styles.insert(col, style);
        self.shift_links_after(col, 1);
    }

    pub fn delete_char(&mut self, col: usize) -> Option<(char, char)> {
        if col >= self.content.len() {
            return None;
        }
        let ch = self.content.remove(col);
        let style = self.styles.remove(col);
        self.remove_link_at(col);
        self.shift_links_after(col, -1);
        Some((ch, style))
    }

    pub fn split_at(&mut self, col: usize) -> Self {
        let col = col.min(self.content.len());

        let right_content = String::from(&self.content[col..]);
        let right_styles = String::from(&self.styles[col..]);

        self.content.truncate(col);
        self.styles.truncate(col);

        let right_links = if let Some(ref mut links) = self.links {
            let mut new_links = BTreeMap::new();
            let keys_to_move: Vec<usize> = links.keys().filter(|&&k| k >= col).copied().collect();
            for k in keys_to_move {
                if let Some(url) = links.remove(&k) {
                    new_links.insert(k - col, url);
                }
            }
            if new_links.is_empty() { None } else { Some(new_links) }
        } else {
            None
        };

        if let Some(ref links) = self.links {
            if links.is_empty() { self.links = None; }
        }

        Self {
            meta: MetaLine::new(),
            content: right_content,
            styles: right_styles,
            links: right_links,
        }
    }

    pub fn append(&mut self, other: &Line) {
        let offset = self.content.len();
        self.content.push_str(&other.content);
        self.styles.push_str(&other.styles);

        if let Some(ref other_links) = other.links {
            let links = self.links.get_or_insert_with(BTreeMap::new);
            for (&col, url) in other_links {
                links.insert(col + offset, url.clone());
            }
        }
    }

    pub fn apply_style_range(&mut self, start: usize, end: usize, style: char) {
        let start = start.min(self.styles.len());
        let end = end.min(self.styles.len());
        if start >= end { return; }

        let mut chars: Vec<char> = self.styles.chars().collect();
        for ch in chars.iter_mut().skip(start).take(end - start) {
            *ch = style;
        }
        self.styles = chars.into_iter().collect();
    }

    pub fn get_style_at(&self, col: usize) -> char {
        self.styles.chars().nth(col).unwrap_or(inline::PLAIN)
    }

    pub fn set_format(&mut self, format: char, level: u8) {
        self.meta.format = format;
        self.meta.level = level;
    }

    fn shift_links_after(&mut self, col: usize, delta: i32) {
        if let Some(ref mut links) = self.links {
            let mut new_links = BTreeMap::new();
            for (&k, v) in links.iter() {
                if k >= col {
                    let new_k = (k as i32 + delta) as usize;
                    new_links.insert(new_k, v.clone());
                } else {
                    new_links.insert(k, v.clone());
                }
            }
            *links = new_links;
        }
    }

    fn remove_link_at(&mut self, col: usize) {
        if let Some(ref mut links) = self.links {
            links.remove(&col);
            if links.is_empty() { self.links = None; }
        }
    }
}

impl Default for Line {
    fn default() -> Self { Self::new() }
}
