use super::{Editor, Mode, Operator, Register};

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
    LineStart,          // 0
    LineEnd,            // $
    FirstNonBlank,      // ^
    GotoTop,            // gg
    GotoBottom,         // G
    HalfPageUp,         // ctrl+u
    HalfPageDown,       // ctrl+d
    // modes
    InsertBefore,    // i
    InsertAfter,     // a
    InsertLineStart, // I
    InsertLineEnd,   // A
    OpenLineBelow,   // o
    OpenLineAbove,   // O
    EnterCommand,    // :
    EnterVisual,     // v
    // editing
    DeleteChar,              // x
    DeleteToLineEnd,         // D
    JoinLines,               // J
    ToggleCase,              // ~
    ReplaceChar,             // r (waits for the replacement char)
    Paste,                   // p (after the cursor)
    PasteBefore,             // P (before the cursor)
    StartOperator(Operator), // d/c/y (starts operator-pending)
    // system
    Save,
    Quit,
}

impl Action {
    /// Whether this action only moves the cursor (used to allow motions
    /// while in Visual mode without triggering edits or mode switches).
    pub fn is_motion(&self) -> bool {
        matches!(
            self,
            Action::MoveLeft
                | Action::MoveRight
                | Action::MoveUp
                | Action::MoveDown
                | Action::WordForward(_)
                | Action::WordBackward(_)
                | Action::WordEnd(_)
                | Action::LineStart
                | Action::LineEnd
                | Action::FirstNonBlank
                | Action::GotoTop
                | Action::GotoBottom
                | Action::HalfPageUp
                | Action::HalfPageDown
        )
    }
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
            Action::EnterVisual => {
                self.anchor = Some((self.position_x, self.position_y));
                self.mode = Mode::Visual;
            }
            Action::DeleteChar => {
                let x = self.position_x as usize;
                let y = self.position_y as usize;
                // nothing to delete on an empty line
                if self.document.line_len(y) > 0 {
                    if let Some(c) = self.document.char_at(x, y) {
                        self.register = Register::Char(c.to_string());
                    }
                    self.document.delete_char(x, y);
                    self.clamp_x_to_row();
                }
            }
            Action::DeleteToLineEnd => {
                self.document
                    .truncate(self.position_x as usize, self.position_y as usize);
                self.clamp_x_to_row();
            }
            Action::LineStart => self.position_x = 0,
            Action::LineEnd => self.position_x = self.current_row_len().saturating_sub(1),
            Action::FirstNonBlank => {
                if let Some(line) = self.document.row(self.position_y as usize) {
                    let x = line.chars().position(|c| !c.is_whitespace()).unwrap_or(0);
                    self.position_x = x as u16;
                }
            }
            Action::OpenLineBelow => {
                let at = (self.position_y as usize + 1).min(self.document.rows_len());
                self.document.insert_row(at, String::new());
                self.position_y = at as u16;
                self.position_x = 0;
                self.mode = Mode::Insert;
            }
            Action::OpenLineAbove => {
                self.document
                    .insert_row(self.position_y as usize, String::new());
                self.position_x = 0;
                self.mode = Mode::Insert;
            }
            Action::JoinLines => {
                self.document.join_below(self.position_y as usize);
                self.clamp_x_to_row();
            }
            Action::ToggleCase => {
                let x = self.position_x as usize;
                let y = self.position_y as usize;
                if let Some(c) = self.document.char_at(x, y) {
                    let toggled = if c.is_uppercase() {
                        c.to_lowercase().next().unwrap_or(c)
                    } else {
                        c.to_uppercase().next().unwrap_or(c)
                    };
                    self.document.replace_char(x, y, toggled);
                    // move right within the line (vim behaviour)
                    if self.current_row_len() > self.position_x.saturating_add(1) {
                        self.position_x += 1;
                    }
                }
            }
            // wait for the next key, which replaces the char under the cursor
            Action::ReplaceChar => self.awaiting_replace = true,
            Action::Paste => self.paste(false),
            Action::PasteBefore => self.paste(true),
            // arm the operator; its target key is handled on the next press
            Action::StartOperator(op) => self.pending_op = Some(op),
            Action::Save => {
                let _ = self.document.save();
            }
            Action::Quit => self.should_quit = true,
        }
    }

    /// Paste the register at the cursor (`p` after, `P` before).
    fn paste(&mut self, before: bool) {
        match self.register.clone() {
            Register::None => {}
            Register::Line(text) => {
                let y = self.position_y as usize;
                let at = if before { y } else { y + 1 };
                self.document.insert_row(at, text);
                self.position_y = at as u16;
                self.position_x = 0;
                self.clamp_y_to_doc();
            }
            Register::Char(text) => {
                let x = self.position_x as usize;
                let y = self.position_y as usize;
                let at = if before {
                    x
                } else {
                    (x + 1).min(self.document.line_len(y))
                };
                self.document.insert_str(at, y, &text);
                // leave the cursor on the last pasted char (single-line case)
                if !text.contains('\n') {
                    self.position_x = (at + text.chars().count()).saturating_sub(1) as u16;
                }
                self.clamp_x_to_row();
            }
        }
    }
}
