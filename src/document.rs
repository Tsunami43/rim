use std::{self, fs::read_to_string, io::Result};

pub struct Document {
    pub rows: Vec<String>,
    pub filename: Option<String>,
    pub dirty: bool,
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

    pub fn remove_line(&mut self, y: u16) {
        if self.rows.get(y as usize).is_some() {
            self.rows.remove(y as usize);
        }
        self.dirty = true;
    }
    pub fn truncate(&mut self, x: u16, y: u16) {
        if let Some(row) = self.rows.get_mut(y as usize) {
            row.truncate(x as usize);
        }
        self.dirty = true;
    }

    pub fn insert_char(&mut self, x: u16, y: u16, ch: char) {
        if self.rows.is_empty() {
            self.rows.push(String::new());
        }
        if let Some(row) = self.rows.get_mut(y as usize) {
            row.insert(x as usize, ch);
        }
        self.dirty = true;
    }

    pub fn delete_char(&mut self, x: u16, y: u16) {
        if let Some(row) = self.rows.get_mut(y as usize) {
            row.remove(x as usize);
        }
        self.dirty = true;
    }
    pub fn line_len(&self, y: u16) -> u16 {
        self.rows.get(y as usize).map_or(0, |row| row.len()) as u16
    }

    pub fn insert_newline(&mut self, x: u16, y: u16) {
        let y = y as usize;
        if y >= self.rows.len() {
            self.rows.push(String::new());
            return;
        }
        let rest = self.rows[y].split_off(x as usize);
        self.rows.insert(y + 1, rest);
        self.dirty = true;
    }

    pub fn join_line(&mut self, y: u16) {
        let y = y as usize;
        if y == 0 || y >= self.rows.len() {
            return;
        }
        let current = self.rows.remove(y);
        self.rows[y - 1].push_str(&current);
        self.dirty = true;
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(name) = &self.filename {
            let contents = self.rows.join("\n");
            std::fs::write(name, contents)?;
            self.dirty = false;
        }
        Ok(())
    }
}
