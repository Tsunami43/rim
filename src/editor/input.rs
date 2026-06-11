use super::{Editor, Mode, Operator, action::Action, command_line::Command, keymap::KeyBind};

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

    /// Apply an operator (`d` or `c`) to the motion given by `key`
    /// (dd/cc, dw/cw, db, de and big variants).
    pub fn apply_operator(&mut self, op: Operator, key: KeyEvent) {
        let x = self.position_x as usize;
        let y = self.position_y as usize;

        // doubled operator -> linewise (dd / cc)
        let doubled = matches!(
            (op, key.code),
            (Operator::Delete, KeyCode::Char('d')) | (Operator::Change, KeyCode::Char('c'))
        );
        if doubled {
            match op {
                Operator::Delete => {
                    self.document.remove_line(y);
                    self.clamp_y_to_doc();
                    self.clamp_x_to_row();
                }
                Operator::Change => {
                    // clear the line but keep it, then insert at column 0
                    self.document.truncate(0, y);
                    self.position_x = 0;
                    self.mode = Mode::Insert;
                }
            }
            return;
        }

        // resolve the target position of the motion
        let target = match key.code {
            KeyCode::Char('w') => Some(self.clamp_target_to_line(self.document.next_word(x, y, false))),
            KeyCode::Char('W') => Some(self.clamp_target_to_line(self.document.next_word(x, y, true))),
            KeyCode::Char('b') => Some(self.document.previous_word(x, y, false)),
            KeyCode::Char('B') => Some(self.document.previous_word(x, y, true)),
            KeyCode::Char('e') => {
                let (ex, ey) = self.document.next_word_end(x, y, false);
                Some((ex + 1, ey)) // `e` is inclusive
            }
            KeyCode::Char('E') => {
                let (ex, ey) = self.document.next_word_end(x, y, true);
                Some((ex + 1, ey))
            }
            _ => None,
        };

        if let Some(target) = target {
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

    /// Delete the current Visual selection (inclusive) and return to Normal.
    fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_bounds() {
            // the selection includes the end cell, so the exclusive end is end.x + 1
            let (nx, ny) = self.document.delete_range(
                (start.0 as usize, start.1 as usize),
                (end.0 as usize + 1, end.1 as usize),
            );
            self.position_x = nx as u16;
            self.position_y = ny as u16;
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

        // 1) waiting for an operator target (dw, dd, cw, cc, ...)
        if let Some(op) = self.pending_op.take() {
            self.apply_operator(op, key);
            return;
        }

        // 2) the gg sequence
        if was_awaiting_g {
            if key.code == KeyCode::Char('g') {
                self.execute_action(Action::GotoTop);
            }
            return;
        }
        if key.code == KeyCode::Char('g') && !key.modifiers.contains(KeyModifiers::CONTROL) {
            self.awaiting_g = true;
            return;
        }

        // 3) single key via the keymap
        let bind = KeyBind::from_event(key);
        if let Some(action) = self.keymap.lookup_normal(&bind) {
            self.execute_action(action);
        }
    }
}
