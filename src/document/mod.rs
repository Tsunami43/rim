use std::{self, fs::read_to_string, io::Result};
mod edit;
mod motion;
mod search;

/// An in-memory text buffer: the lines plus file metadata.
#[derive(Clone)]
pub struct Document {
    rows: Vec<String>,
    filename: Option<String>,
    dirty: bool,
}

impl Document {
    /// Load a file into a new document.
    pub fn open(filename: &str) -> Result<Self> {
        let text = read_to_string(filename)?;
        let rows = text.lines().map(|line| line.to_string()).collect();
        Ok(Self {
            rows,
            filename: Some(filename.to_string()),
            dirty: false,
        })
    }

    /// An empty document with no backing file.
    pub fn empty() -> Self {
        Self {
            rows: Vec::new(),
            filename: None,
            dirty: false,
        }
    }

    /// Write the buffer back to its file and clear the dirty flag.
    pub fn save(&mut self) -> Result<()> {
        if let Some(name) = &self.filename {
            let contents = self.rows.join("\n");
            std::fs::write(name, contents)?;
            self.dirty = false;
        }
        Ok(())
    }

    /// The line at index `i`, if it exists.
    pub fn row(&self, i: usize) -> Option<&str> {
        self.rows.get(i).map(|s| s.as_str())
    }

    /// The backing file name, if any.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Whether the buffer has no lines.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Whether there are unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Number of lines.
    pub fn rows_len(&self) -> usize {
        self.rows.len()
    }

    /// Length (in bytes) of line `i`, or 0 if it doesn't exist.
    pub fn line_len(&self, i: usize) -> usize {
        self.rows.get(i).map_or(0, |row| row.len())
    }

    /// The character at `(x, y)`, if it exists.
    pub fn char_at(&self, x: usize, y: usize) -> Option<char> {
        self.rows.get(y).and_then(|row| row.chars().nth(x))
    }
}

#[cfg(test)]
impl Document {
    /// Build a document directly from lines (tests only).
    pub(crate) fn from_lines(lines: &[&str]) -> Self {
        Self {
            rows: lines.iter().map(|s| s.to_string()).collect(),
            filename: None,
            dirty: false,
        }
    }

    /// The lines as `&str` (tests only).
    pub(crate) fn lines(&self) -> Vec<&str> {
        self.rows.iter().map(|s| s.as_str()).collect()
    }
}
