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

    fn class_of(&self, c: char, big: bool) -> u8 {
        if c.is_whitespace() {
            0
        } else if big {
            1
        } else if c.is_alphanumeric() || c == '_' {
            1
        } else {
            2
        }
    }

    pub fn next_word(&self, x: u16, y: u16, big: bool) -> (u16, u16) {
        let line = match self.rows.get(y as usize) {
            Some(l) => l,
            None => return (x, y),
        };
        let chars: Vec<char> = line.chars().collect();
        let n = chars.len();
        let mut i = x as usize;

        if i < n {
            let cls = self.class_of(chars[i], big);
            if cls != 0 {
                while i < n && self.class_of(chars[i], big) == cls {
                    i += 1;
                }
            }
            while i < n && self.class_of(chars[i], big) == 0 {
                i += 1;
            }
        }

        if i >= n {
            let last = self.rows.len().saturating_sub(1) as u16;
            if y < last {
                return (0, y + 1);
            }
            return (n.saturating_sub(1) as u16, y);
        }
        (i as u16, y)
    }

    pub fn next_word_end(&self, x: u16, y: u16, big: bool) -> (u16, u16) {
        let mut y = y;
        let mut i = x as usize + 1;

        loop {
            let chars: Vec<char> = match self.rows.get(y as usize) {
                Some(l) => l.chars().collect(),
                None => return (x, y),
            };
            let n = chars.len();

            if i >= n {
                let last = self.rows.len().saturating_sub(1) as u16;
                if y >= last {
                    return (n.saturating_sub(1) as u16, y);
                }
                y += 1;
                i = 0;
                continue;
            }

            while i < n && self.class_of(chars[i], big) == 0 {
                i += 1;
            }
            if i >= n {
                continue;
            }
            let cls = self.class_of(chars[i], big);
            while i + 1 < n && self.class_of(chars[i + 1], big) == cls {
                i += 1;
            }
            return (i as u16, y);
        }
    }

    pub fn previous_word(&self, x: u16, y: u16, big: bool) -> (u16, u16) {
        let mut y = y;
        let mut i = x as usize;

        loop {
            if i == 0 {
                if y == 0 {
                    return (0, 0);
                }
                y -= 1;
                i = self.line_len(y) as usize;
                continue;
            }

            let chars: Vec<char> = match self.rows.get(y as usize) {
                Some(l) => l.chars().collect(),
                None => return (x, y),
            };

            i -= 1;
            while i > 0 && self.class_of(chars[i], big) == 0 {
                i -= 1;
            }
            if self.class_of(chars[i], big) == 0 {
                continue;
            }
            let cls = self.class_of(chars[i], big);
            while i > 0 && self.class_of(chars[i - 1], big) == cls {
                i -= 1;
            }
            return (i as u16, y);
        }
    }
}
