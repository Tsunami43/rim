use super::Document;

/// Index of the first match of `pat` in `chars` at or after `start`.
fn find_in_row(chars: &[char], pat: &[char], start: usize) -> Option<usize> {
    if pat.is_empty() || pat.len() > chars.len() {
        return None;
    }
    let mut i = start;
    while i + pat.len() <= chars.len() {
        if &chars[i..i + pat.len()] == pat {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Index of the last match of `pat` in `chars` that starts before `before`.
fn rfind_in_row(chars: &[char], pat: &[char], before: usize) -> Option<usize> {
    if pat.is_empty() || pat.len() > chars.len() || before == 0 {
        return None;
    }
    let max_start = (chars.len() - pat.len()).min(before - 1);
    (0..=max_start)
        .rev()
        .find(|&start| &chars[start..start + pat.len()] == pat)
}

impl Document {
    /// Next match of `pattern` after `from`, wrapping around the document.
    pub fn find(&self, pattern: &str, from: (usize, usize)) -> Option<(usize, usize)> {
        if pattern.is_empty() || self.rows.is_empty() {
            return None;
        }
        let pat: Vec<char> = pattern.chars().collect();
        let n = self.rows.len();
        let (fx, fy) = from;

        for i in 0..=n {
            let y = (fy + i) % n;
            // on the starting row, look strictly after the cursor;
            // after wrapping all the way round, scan it from the start
            let start = if i == 0 { fx + 1 } else { 0 };
            let chars: Vec<char> = self.rows[y].chars().collect();
            if let Some(x) = find_in_row(&chars, &pat, start) {
                return Some((x, y));
            }
        }
        None
    }

    /// Previous match of `pattern` before `from`, wrapping around the document.
    pub fn rfind(&self, pattern: &str, from: (usize, usize)) -> Option<(usize, usize)> {
        if pattern.is_empty() || self.rows.is_empty() {
            return None;
        }
        let pat: Vec<char> = pattern.chars().collect();
        let n = self.rows.len();
        let (fx, fy) = from;

        for i in 0..=n {
            let y = ((fy as isize - i as isize).rem_euclid(n as isize)) as usize;
            let chars: Vec<char> = self.rows[y].chars().collect();
            let before = if i == 0 { fx } else { chars.len() };
            if let Some(x) = rfind_in_row(&chars, &pat, before) {
                return Some((x, y));
            }
        }
        None
    }

    /// The word (alphanumeric + `_`) under `(x, y)`, if any.
    pub fn word_at(&self, x: usize, y: usize) -> Option<String> {
        let chars: Vec<char> = self.rows.get(y)?.chars().collect();
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if x >= chars.len() || !is_word(chars[x]) {
            return None;
        }
        let mut start = x;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = x;
        while end + 1 < chars.len() && is_word(chars[end + 1]) {
            end += 1;
        }
        Some(chars[start..=end].iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    #[test]
    fn find_forward_and_wrap() {
        let d = Document::from_lines(&["foo bar", "baz bar"]);
        assert_eq!(d.find("bar", (0, 0)), Some((4, 0)));
        assert_eq!(d.find("bar", (4, 0)), Some((4, 1))); // next one, on line 1
        assert_eq!(d.find("bar", (4, 1)), Some((4, 0))); // wraps back to line 0
        assert_eq!(d.find("nope", (0, 0)), None);
    }

    #[test]
    fn rfind_backward_and_wrap() {
        let d = Document::from_lines(&["foo bar", "baz bar"]);
        assert_eq!(d.rfind("bar", (4, 1)), Some((4, 0)));
        assert_eq!(d.rfind("bar", (4, 0)), Some((4, 1))); // wraps to line 1
    }

    #[test]
    fn word_under_cursor() {
        let d = Document::from_lines(&["foo bar_baz qux"]);
        assert_eq!(d.word_at(5, 0).as_deref(), Some("bar_baz"));
        assert_eq!(d.word_at(3, 0), None); // on the space
    }
}
