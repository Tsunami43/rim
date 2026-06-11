use super::{Editor, Mode, Operator, Register, action::Action, command_line::Command, keymap::KeyBind};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Editor {
    /// Run the typed `:` command (`:w`, `:q`, `:wq`).
    pub fn execute_command(&mut self) {
        match self.command_line.parse() {
            Command::Save => {
                let _ = self.document.save();
            }
            Command::Quit => self.should_quit = true,
            Command::SaveQuit => {
                let _ = self.document.save();
                self.should_quit = true;
            }
            Command::Unknown => {}
        }
        self.command_line.clear();
    }

    /// Handle a key in Command mode (typing after `:`).
    pub fn handler_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_line.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if self.command_line.is_empty() {
                    self.mode = Mode::Normal;
                } else {
                    self.command_line.pop();
                }
            }
            KeyCode::Enter => {
                self.execute_command();
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                self.command_line.push(c);
            }
            _ => {}
        }
    }

    /// Handle a key in Search mode (typing the pattern after `/`).
    pub fn handler_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_line.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if self.command_line.is_empty() {
                    self.mode = Mode::Normal;
                } else {
                    self.command_line.pop();
                }
            }
            KeyCode::Enter => {
                self.last_search = self.command_line.as_str().to_string();
                self.command_line.clear();
                self.mode = Mode::Normal;
                self.search_next();
            }
            KeyCode::Char(c) => self.command_line.push(c),
            _ => {}
        }
    }

    /// Handle a key in Insert mode (text input, Backspace, Enter, Esc).
    pub fn handler_insert(&mut self, key: KeyEvent) {
        match key.code {
            // Switch mode to Normal
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                if self.position_x > 0 {
                    self.document
                        .delete_char((self.position_x - 1) as usize, self.position_y as usize);
                    self.position_x -= 1;
                } else if self.position_y > 0 {
                    let prev_len = self.document.line_len((self.position_y - 1) as usize) as u16;
                    self.document.join_line(self.position_y as usize);
                    self.position_y -= 1;
                    self.position_x = prev_len;
                }
            }
            KeyCode::Enter => {
                self.document
                    .insert_newline(self.position_x as usize, self.position_y as usize);
                self.position_y += 1;
                self.position_x = 0;
            }
            KeyCode::Char(c) => {
                self.document
                    .insert_char(self.position_x as usize, self.position_y as usize, c);
                self.position_x += 1;
            }
            _ => {}
        }
    }

    /// For word-forward operators (`dw`/`cw`): keep the deleted range inside
    /// the current line, unless the line is empty (vim does not join lines on
    /// `dw` over the last word).
    fn clamp_target_to_line(&self, target: (usize, usize)) -> (usize, usize) {
        let y = self.position_y as usize;
        let len = self.document.line_len(y);
        if target.1 != y && len > 0 {
            (len, y)
        } else {
            target
        }
    }

    /// Apply an operator (`d`/`c`/`y`) to the motion given by `key`, repeated
    /// `n` times (dd/cc/yy linewise, dw/cw/db/de and big variants).
    pub fn apply_operator(&mut self, op: Operator, key: KeyEvent, n: usize) {
        let x = self.position_x as usize;
        let y = self.position_y as usize;

        // d/c mutate the buffer, so snapshot for undo (yank does not)
        if op != Operator::Yank {
            self.push_undo();
        }

        // doubled operator -> linewise over `n` lines (dd / cc / yy)
        let doubled = matches!(
            (op, key.code),
            (Operator::Delete, KeyCode::Char('d'))
                | (Operator::Change, KeyCode::Char('c'))
                | (Operator::Yank, KeyCode::Char('y'))
        );
        if doubled {
            let last = (y + n).min(self.document.rows_len());
            let text = (y..last)
                .map(|i| self.document.row(i).unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n");
            self.register = Register::Line(text);
            match op {
                Operator::Delete => {
                    for _ in y..last {
                        self.document.remove_line(y);
                    }
                    self.clamp_y_to_doc();
                    self.clamp_x_to_row();
                }
                Operator::Change => {
                    // remove the extra lines, clear the current one, then insert
                    for _ in (y + 1)..last {
                        self.document.remove_line(y + 1);
                    }
                    self.document.truncate(0, y);
                    self.position_x = 0;
                    self.mode = Mode::Insert;
                }
                Operator::Yank => {}
            }
            return;
        }

        // resolve the target by applying the motion `n` times
        let mut tx = x;
        let mut ty = y;
        let mut inclusive = false;
        let mut word_forward = false;
        for _ in 0..n {
            let (mx, my) = match key.code {
                KeyCode::Char('w') => {
                    word_forward = true;
                    self.document.next_word(tx, ty, false)
                }
                KeyCode::Char('W') => {
                    word_forward = true;
                    self.document.next_word(tx, ty, true)
                }
                KeyCode::Char('b') => self.document.previous_word(tx, ty, false),
                KeyCode::Char('B') => self.document.previous_word(tx, ty, true),
                KeyCode::Char('e') => {
                    inclusive = true;
                    self.document.next_word_end(tx, ty, false)
                }
                KeyCode::Char('E') => {
                    inclusive = true;
                    self.document.next_word_end(tx, ty, true)
                }
                _ => return, // not a motion -> cancel the operator
            };
            tx = mx;
            ty = my;
        }

        let mut target = (tx, ty);
        if inclusive {
            target = (target.0 + 1, target.1);
        }
        // single `dw`/`cw` must not join lines; for counts allow crossing
        if word_forward && n == 1 {
            target = self.clamp_target_to_line(target);
        }

        self.register = Register::Char(self.document.text_in_range((x, y), target));
        if op == Operator::Yank {
            // move the cursor to the start of the yanked range
            let start = if (target.1, target.0) < (y, x) { target } else { (x, y) };
            self.position_x = start.0 as u16;
            self.position_y = start.1 as u16;
            self.clamp_x_to_row();
        } else {
            let (nx, ny) = self.document.delete_range((x, y), target);
            self.position_x = nx as u16;
            self.position_y = ny as u16;
            self.clamp_x_to_row();
            if op == Operator::Change {
                self.mode = Mode::Insert;
            }
        }
    }

    /// Handle a key in Visual mode: motions extend the selection,
    /// `d`/`x` delete it, `Esc`/`v` leave Visual mode.
    pub fn handler_visual(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.anchor = None;
                self.mode = Mode::Normal;
            }
            KeyCode::Char('d') | KeyCode::Char('x') => self.delete_selection(),
            KeyCode::Char('y') => self.yank_selection(),
            _ => {
                let bind = KeyBind::from_event(key);
                if let Some(action) = self.keymap.lookup_normal(&bind)
                    && action.is_motion()
                {
                    self.execute_action(action);
                }
            }
        }
    }

    /// Inclusive selection range as exclusive document coordinates.
    fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let (start, end) = self.selection_bounds()?;
        Some((
            (start.0 as usize, start.1 as usize),
            (end.0 as usize + 1, end.1 as usize),
        ))
    }

    /// Delete the current Visual selection (inclusive) and return to Normal.
    fn delete_selection(&mut self) {
        if let Some((from, to)) = self.selection_range() {
            self.push_undo();
            self.register = Register::Char(self.document.text_in_range(from, to));
            let (nx, ny) = self.document.delete_range(from, to);
            self.position_x = nx as u16;
            self.position_y = ny as u16;
            self.clamp_x_to_row();
        }
        self.anchor = None;
        self.mode = Mode::Normal;
    }

    /// Yank the current Visual selection and return to Normal.
    fn yank_selection(&mut self) {
        if let Some((from, to)) = self.selection_range() {
            self.register = Register::Char(self.document.text_in_range(from, to));
            self.position_x = from.0 as u16;
            self.position_y = from.1 as u16;
            self.clamp_x_to_row();
        }
        self.anchor = None;
        self.mode = Mode::Normal;
    }

    /// Handle a key in Normal mode: operator-pending and `gg` first,
    /// then a single-key lookup in the keymap.
    pub fn handler_normal(&mut self, key: KeyEvent) {
        let was_awaiting_g = self.awaiting_g;
        self.awaiting_g = false;

        // r: the next key replaces the char under the cursor
        if self.awaiting_replace {
            self.awaiting_replace = false;
            if let KeyCode::Char(c) = key.code {
                self.document
                    .replace_char(self.position_x as usize, self.position_y as usize, c);
            }
            return;
        }

        // 1) waiting for an operator target (dw, dd, cw, cc, 2dw, ...)
        if let Some(op) = self.pending_op.take() {
            let n = self.count.take().unwrap_or(1);
            self.apply_operator(op, key, n);
            return;
        }

        // 2) the gg sequence
        if was_awaiting_g {
            if key.code == KeyCode::Char('g') {
                self.execute_action(Action::GotoTop);
            }
            self.count = None;
            return;
        }

        // 3) accumulate a numeric count prefix (`0` alone is a motion, not a count)
        if let KeyCode::Char(c) = key.code
            && c.is_ascii_digit()
            && !(c == '0' && self.count.is_none())
        {
            let digit = c.to_digit(10).unwrap_or(0) as usize;
            self.count = Some(self.count.unwrap_or(0) * 10 + digit);
            return;
        }

        if key.code == KeyCode::Char('g') && !key.modifiers.contains(KeyModifiers::CONTROL) {
            self.awaiting_g = true;
            return;
        }

        // 4) single key via the keymap, repeated `count` times
        let n = self.count.take().unwrap_or(1);
        let bind = KeyBind::from_event(key);
        if let Some(action) = self.keymap.lookup_normal(&bind) {
            // an operator carries the count forward to its motion (e.g. 3dw)
            if let Action::StartOperator(op) = action {
                self.pending_op = Some(op);
                self.count = Some(n);
            } else {
                for _ in 0..n {
                    self.execute_action(action);
                }
            }
        }
    }
}
