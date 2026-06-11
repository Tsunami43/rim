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

    /// Handle the motion key after `d` (dd, dw, db, de and big variants).
    pub fn handler_pending_d(&mut self, key: KeyEvent) {
        match key.code {
            // Delete current row
            KeyCode::Char('d') => {
                self.document.remove_line(self.position_y as usize);
                self.clamp_y_to_doc();
                self.clamp_x_to_row();
            }
            KeyCode::Char('b') => {
                let target = self.document.previous_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }
            KeyCode::Char('B') => {
                let target = self.document.previous_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }
            KeyCode::Char('w') => {
                let target = self.document.next_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }

            KeyCode::Char('W') => {
                let target = self.document.next_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }
            KeyCode::Char('e') => {
                let target = self.document.next_word_end(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }
            KeyCode::Char('E') => {
                let target = self.document.next_word_end(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                let (nx, ny) = self
                    .document
                    .delete_range((self.position_x as usize, self.position_y as usize), target);
                self.position_x = nx as u16;
                self.position_y = ny as u16;
                self.clamp_x_to_row();
            }
            _ => {}
        }
    }

    /// Handle a key in Normal mode: operator-pending and `gg` first,
    /// then a single-key lookup in the keymap.
    pub fn handler_normal(&mut self, key: KeyEvent) {
        let was_awaiting_g = self.awaiting_g;
        self.awaiting_g = false;

        // 1) waiting for an operator target (dw, dd, ...)
        if let Some(op) = self.pending_op.take() {
            match op {
                Operator::Delete => self.handler_pending_d(key),
            }
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
