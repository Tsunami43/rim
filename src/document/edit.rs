use super::Document;

impl Document {
    /// Insert `ch` at column `x` of line `y`.
    pub fn insert_char(&mut self, x: usize, y: usize, ch: char) {
        if self.is_empty() {
            self.rows.push(String::new());
        }
        if let Some(row) = self.rows.get_mut(y) {
            row.insert(x, ch);
        }
        self.dirty = true;
    }

    /// Delete the character at column `x` of line `y`.
    pub fn delete_char(&mut self, x: usize, y: usize) {
        if let Some(row) = self.rows.get_mut(y) {
            row.remove(x);
        }
        self.dirty = true;
    }

    /// Append line `y` onto the end of line `y - 1`, removing line `y`.
    pub fn join_line(&mut self, y: usize) {
        if y == 0 || y >= self.rows.len() {
            return;
        }
        let current = self.rows.remove(y);
        self.rows[y - 1].push_str(&current);
        self.dirty = true;
    }

    /// Split line `y` at column `x`, moving the tail onto a new line below.
    pub fn insert_newline(&mut self, x: usize, y: usize) {
        if y >= self.rows.len() {
            self.rows.push(String::new());
            return;
        }
        let rest = self.rows[y].split_off(x);
        self.rows.insert(y + 1, rest);
        self.dirty = true;
    }

    /// Remove line `y` entirely.
    pub fn remove_line(&mut self, y: usize) {
        if self.rows.get(y).is_some() {
            self.rows.remove(y);
        }
        self.dirty = true;
    }

    /// Cut line `y` down to column `x` (delete from the cursor to end of line).
    pub fn truncate(&mut self, x: usize, y: usize) {
        if let Some(row) = self.rows.get_mut(y) {
            row.truncate(x);
        }
        self.dirty = true;
    }

    /// Delete the half-open range `[from, to)` (may span lines, joining the
    /// edges). Returns the new cursor position (the start of the range).
    pub fn delete_range(&mut self, from: (usize, usize), to: (usize, usize)) -> (usize, usize) {
        let (start, end) = if (from.1, from.0) <= (to.1, to.0) {
            (from, to)
        } else {
            (to, from)
        };
        let (sx, sy) = (start.0, start.1);
        let (ex, ey) = (end.0, end.1);

        if sy == ey {
            // single line: keep head before `sx` and tail from `ex`
            if let Some(row) = self.rows.get_mut(sy) {
                let head: String = row.chars().take(sx).collect();
                let tail: String = row.chars().skip(ex).collect();
                *row = head + &tail;
            }
        } else if ey < self.rows.len() {
            // multi-line: join head of the first line with tail of the last
            let head: String = self.rows[sy].chars().take(sx).collect();
            let tail: String = self.rows[ey].chars().skip(ex).collect();
            self.rows.drain((sy + 1)..=ey);
            self.rows[sy] = head + &tail;
        }
        self.dirty = true;
        (start.0, start.1)
    }
}
