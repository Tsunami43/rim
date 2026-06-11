use super::{Editor, Mode, Operator, command_line::Command};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    terminal::size,
};

impl Editor {
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

    pub fn handler_normal(&mut self, key: KeyEvent) {
        let was_awaiting_g = self.awaiting_g;
        self.awaiting_g = false;

        if let Some(op) = self.pending_op.take() {
            match op {
                Operator::Delete => self.handler_pending_d(key),
            }
            return;
        }

        match key.code {
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
            }
            // Save document (ctrl+s)
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = self.document.save();
            }
            // Exit (ctrl+q)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            // Switch mode to Insert (a)
            KeyCode::Char('a') => {
                self.position_x += 1;
                self.mode = Mode::Insert
            }
            // Switch mode to start row Insert (shift + a / A)
            KeyCode::Char('A') => {
                self.position_x = self.current_row_len();
                self.mode = Mode::Insert
            }
            // Switch mode to Insert (i)
            KeyCode::Char('i') => self.mode = Mode::Insert,
            // Switch mode to start row Insert (shift + i / I)
            KeyCode::Char('I') => {
                self.position_x = 0;
                self.mode = Mode::Insert
            }
            // Move left (h)
            KeyCode::Char('h') => self.position_x = self.position_x.saturating_sub(1),
            // Move down (j)
            KeyCode::Char('j') => {
                if (self.position_y as usize) + 1 < self.document.rows_len() {
                    self.position_y = self.position_y.saturating_add(1)
                }
                self.clamp_x_to_row();
            }
            // Move up (k)
            KeyCode::Char('k') => {
                self.position_y = self.position_y.saturating_sub(1);
                self.clamp_x_to_row();
            }
            // Move right (l)
            KeyCode::Char('l') if self.current_row_len() > self.position_x.saturating_add(1) => {
                self.position_x = self.position_x.saturating_add(1)
            }
            // Move to start document (gg)
            KeyCode::Char('g') if was_awaiting_g => {
                self.position_y = 0;
                self.clamp_x_to_row();
            }
            // Move to end document (G)
            KeyCode::Char('G') => {
                self.position_y = self.document.rows_len().saturating_sub(1) as u16;
                self.clamp_x_to_row();
            }
            // Move to half page up (ctrl + u)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let (_, rows) = size().unwrap();
                self.position_y = self.position_y.saturating_sub(rows / 2);
                self.clamp_x_to_row();
            }
            // Move to half page down (ctrl + d)
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let (_, rows) = size().unwrap();
                let move_y = rows / 2;
                let max_y = self.document.rows_len() as u16;

                let new_y = self.position_y.saturating_add(move_y);

                if new_y >= max_y {
                    self.position_y = max_y.saturating_sub(1);
                } else {
                    self.position_y = new_y;
                }
                self.clamp_x_to_row();
            }
            // Switch state g
            KeyCode::Char('g') => self.awaiting_g = true,
            // Delete under cursor
            KeyCode::Char('x') if self.document.line_len(self.position_y as usize) > 0 => {
                self.document
                    .delete_char(self.position_x as usize, self.position_y as usize);
                self.clamp_x_to_row();
            }
            // Delete truncate to cursor
            KeyCode::Char('D') => {
                self.document
                    .truncate(self.position_x as usize, self.position_y as usize);
                self.clamp_x_to_row();
            }

            // Previous word (foo.bar)
            KeyCode::Char('b') => {
                let (x, y) = self.document.previous_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Previous word (foo bar)
            KeyCode::Char('B') => {
                let (x, y) = self.document.previous_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Next word (foo.bar)
            KeyCode::Char('w') => {
                let (x, y) = self.document.next_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Next word (foo bar)
            KeyCode::Char('W') => {
                let (x, y) = self.document.next_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Next word end (foo.bar)
            KeyCode::Char('e') => {
                let (x, y) = self.document.next_word_end(
                    self.position_x as usize,
                    self.position_y as usize,
                    false,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Next word end (foo bar)
            KeyCode::Char('E') => {
                let (x, y) = self.document.next_word_end(
                    self.position_x as usize,
                    self.position_y as usize,
                    true,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            // Switch state d
            KeyCode::Char('d') => self.pending_op = Some(Operator::Delete),
            _ => {}
        }
    }
}
