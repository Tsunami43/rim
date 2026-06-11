use super::Document;

impl Document {
    fn class_of(&self, c: char, big: bool) -> u8 {
        if c.is_whitespace() {
            0
        } else if !big && !(c.is_alphanumeric() || c == '_') {
            2
        } else {
            1
        }
    }

    pub fn next_word(&self, x: usize, y: usize, big: bool) -> (usize, usize) {
        let line = match self.rows.get(y) {
            Some(l) => l,
            None => return (x, y),
        };
        let chars: Vec<char> = line.chars().collect();
        let n = chars.len();
        let mut i = x;

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
            let last = self.rows.len().saturating_sub(1);
            if y < last {
                return (0, y + 1);
            }
            return (n.saturating_sub(1), y);
        }
        (i, y)
    }

    pub fn next_word_end(&self, x: usize, y: usize, big: bool) -> (usize, usize) {
        let mut y = y;
        let mut i = x + 1;

        loop {
            let chars: Vec<char> = match self.rows.get(y) {
                Some(l) => l.chars().collect(),
                None => return (x, y),
            };
            let n = chars.len();

            if i >= n {
                let last = self.rows.len().saturating_sub(1);
                if y >= last {
                    return (n.saturating_sub(1), y);
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
            return (i, y);
        }
    }

    pub fn previous_word(&self, x: usize, y: usize, big: bool) -> (usize, usize) {
        let mut y = y;
        let mut i = x;

        loop {
            if i == 0 {
                if y == 0 {
                    return (0, 0);
                }
                y -= 1;
                i = self.line_len(y);
                continue;
            }

            let chars: Vec<char> = match self.rows.get(y) {
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
            return (i, y);
        }
    }
}
