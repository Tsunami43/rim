use super::{Editor, Mode, Operator};

use crossterm::terminal::size;

/// Every editor action — "what to do", decoupled from any key.
/// The keymap maps a key to one of these actions, while all the
/// execution logic lives in `execute_action`.
#[derive(Clone, Copy)]
pub enum Action {
    // motions
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    WordForward(bool),  // bool = big (W instead of w)
    WordBackward(bool), // B instead of b
    WordEnd(bool),      // E instead of e
    GotoTop,            // gg
    GotoBottom,         // G
    HalfPageUp,         // ctrl+u
    HalfPageDown,       // ctrl+d
    // modes
    InsertBefore,    // i
    InsertAfter,     // a
    InsertLineStart, // I
    InsertLineEnd,   // A
    EnterCommand,    // :
    // editing
    DeleteChar,              // x
    DeleteToLineEnd,         // D
    StartOperator(Operator), // d (starts operator-pending)
    // system
    Save,
    Quit,
}

impl Editor {
    /// The single place where an action turns into an effect.
    pub fn execute_action(&mut self, action: Action) {
        match action {
            Action::MoveLeft => self.position_x = self.position_x.saturating_sub(1),
            Action::MoveRight => {
                // do not move past the last character of the line
                if self.current_row_len() > self.position_x.saturating_add(1) {
                    self.position_x = self.position_x.saturating_add(1);
                }
            }
            Action::MoveUp => {
                self.position_y = self.position_y.saturating_sub(1);
                self.clamp_x_to_row();
            }
            Action::MoveDown => {
                // only move down while there is a next line
                if (self.position_y as usize) + 1 < self.document.rows_len() {
                    self.position_y = self.position_y.saturating_add(1);
                }
                self.clamp_x_to_row();
            }
            Action::WordForward(big) => {
                let (x, y) = self.document.next_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    big,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            Action::WordBackward(big) => {
                let (x, y) = self.document.previous_word(
                    self.position_x as usize,
                    self.position_y as usize,
                    big,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            Action::WordEnd(big) => {
                let (x, y) = self.document.next_word_end(
                    self.position_x as usize,
                    self.position_y as usize,
                    big,
                );
                self.position_x = x as u16;
                self.position_y = y as u16;
            }
            Action::GotoTop => {
                self.position_y = 0;
                self.clamp_x_to_row();
            }
            Action::GotoBottom => {
                self.position_y = self.document.rows_len().saturating_sub(1) as u16;
                self.clamp_x_to_row();
            }
            Action::HalfPageUp => {
                let (_, rows) = size().unwrap();
                self.position_y = self.position_y.saturating_sub(rows / 2);
                self.clamp_x_to_row();
            }
            Action::HalfPageDown => {
                let (_, rows) = size().unwrap();
                let max_y = self.document.rows_len() as u16;
                let new_y = self.position_y.saturating_add(rows / 2);
                self.position_y = if new_y >= max_y {
                    max_y.saturating_sub(1)
                } else {
                    new_y
                };
                self.clamp_x_to_row();
            }
            Action::InsertBefore => self.mode = Mode::Insert,
            Action::InsertAfter => {
                self.position_x += 1;
                self.mode = Mode::Insert;
            }
            Action::InsertLineStart => {
                self.position_x = 0;
                self.mode = Mode::Insert;
            }
            Action::InsertLineEnd => {
                self.position_x = self.current_row_len();
                self.mode = Mode::Insert;
            }
            Action::EnterCommand => self.mode = Mode::Command,
            Action::DeleteChar => {
                // nothing to delete on an empty line
                if self.document.line_len(self.position_y as usize) > 0 {
                    self.document
                        .delete_char(self.position_x as usize, self.position_y as usize);
                    self.clamp_x_to_row();
                }
            }
            Action::DeleteToLineEnd => {
                self.document
                    .truncate(self.position_x as usize, self.position_y as usize);
                self.clamp_x_to_row();
            }
            // arm the operator; its target key is handled on the next press
            Action::StartOperator(op) => self.pending_op = Some(op),
            Action::Save => {
                let _ = self.document.save();
            }
            Action::Quit => self.should_quit = true,
        }
    }
}
