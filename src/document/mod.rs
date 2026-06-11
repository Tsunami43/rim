use std::{self, fs::read_to_string, io::Result};
mod edit;
mod motion;
mod search;

pub struct Document {
    rows: Vec<String>,
    filename: Option<String>,
    dirty: bool,
}

impl Document {
    pub fn open(filename: &str) -> Result<Self> {
        let text = read_to_string(filename)?;
        let rows = text.lines().map(|line| line.to_string()).collect();
        Ok(Self {
            rows,
            filename: Some(filename.to_string()),
            dirty: false,
        })
    }

    pub fn empty() -> Self {
        Self {
            rows: Vec::new(),
            filename: None,
            dirty: false,
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(name) = &self.filename {
            let contents = self.rows.join("\n");
            std::fs::write(name, contents)?;
            self.dirty = false;
        }
        Ok(())
    }

    pub fn row(&self, i: usize) -> Option<&str> {
        self.rows.get(i).map(|s| s.as_str())
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn rows_len(&self) -> usize {
        self.rows.len()
    }

    pub fn line_len(&self, i: usize) -> usize {
        self.rows.get(i).map_or(0, |row| row.len())
    }
}
