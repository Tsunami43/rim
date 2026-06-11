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

    /// Insert `text` as a new line at index `y` (clamped to the end).
    pub fn insert_row(&mut self, y: usize, text: String) {
        let y = y.min(self.rows.len());
        self.rows.insert(y, text);
        self.dirty = true;
    }

    /// Replace the character at `(x, y)` with `ch` (no-op if out of range).
    pub fn replace_char(&mut self, x: usize, y: usize, ch: char) {
        if let Some(row) = self.rows.get_mut(y) {
            let mut chars: Vec<char> = row.chars().collect();
            if x < chars.len() {
                chars[x] = ch;
                *row = chars.into_iter().collect();
                self.dirty = true;
            }
        }
    }

    /// Join line `y + 1` onto line `y`, separated by a single space (vim's J).
    pub fn join_below(&mut self, y: usize) {
        if y + 1 >= self.rows.len() {
            return;
        }
        let next = self.rows.remove(y + 1);
        let trimmed = next.trim_start();
        if !self.rows[y].is_empty() && !trimmed.is_empty() {
            self.rows[y].push(' ');
        }
        self.rows[y].push_str(trimmed);
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    #[test]
    fn insert_and_delete_char() {
        let mut d = Document::from_lines(&["ac"]);
        d.insert_char(1, 0, 'b');
        assert_eq!(d.lines(), vec!["abc"]);
        d.delete_char(1, 0);
        assert_eq!(d.lines(), vec!["ac"]);
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut d = Document::from_lines(&["foobar"]);
        d.insert_newline(3, 0);
        assert_eq!(d.lines(), vec!["foo", "bar"]);
    }

    #[test]
    fn join_line_merges_into_previous() {
        let mut d = Document::from_lines(&["foo", "bar"]);
        d.join_line(1);
        assert_eq!(d.lines(), vec!["foobar"]);
    }

    #[test]
    fn truncate_cuts_to_end() {
        let mut d = Document::from_lines(&["hello world"]);
        d.truncate(5, 0);
        assert_eq!(d.lines(), vec!["hello"]);
    }

    #[test]
    fn delete_range_single_line() {
        let mut d = Document::from_lines(&["foo bar baz"]);
        let pos = d.delete_range((0, 0), (4, 0)); // delete "foo "
        assert_eq!(d.lines(), vec!["bar baz"]);
        assert_eq!(pos, (0, 0));
    }

    #[test]
    fn delete_range_multi_line_joins() {
        let mut d = Document::from_lines(&["hello", "world"]);
        // from middle of line 0 to middle of line 1 -> the two lines merge
        d.delete_range((2, 0), (2, 1));
        assert_eq!(d.lines(), vec!["herld"]);
    }

    #[test]
    fn delete_range_normalizes_reversed_input() {
        let mut d = Document::from_lines(&["foo bar baz"]);
        // passing the larger position first still deletes [4, 8)
        let pos = d.delete_range((8, 0), (4, 0));
        assert_eq!(d.lines(), vec!["foo baz"]);
        assert_eq!(pos, (4, 0));
    }

    #[test]
    fn replace_char_in_place() {
        let mut d = Document::from_lines(&["cat"]);
        d.replace_char(0, 0, 'b');
        assert_eq!(d.lines(), vec!["bat"]);
    }

    #[test]
    fn join_below_adds_space() {
        let mut d = Document::from_lines(&["foo", "   bar"]);
        d.join_below(0);
        assert_eq!(d.lines(), vec!["foo bar"]);
    }

    #[test]
    fn insert_row_inserts_line() {
        let mut d = Document::from_lines(&["a", "b"]);
        d.insert_row(1, "x".to_string());
        assert_eq!(d.lines(), vec!["a", "x", "b"]);
    }
}
