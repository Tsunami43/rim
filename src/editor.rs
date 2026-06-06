use std::{
    self,
    io::{stdout, Result, Write},
};

use crate::document::Document;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    queue,
    terminal::{size, Clear, ClearType},
};

#[derive(Clone, Debug, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
}

pub struct Editor {
    should_quit: bool,
    awaiting_g: bool,
    document: Document,
    mode: Mode,
    position_x: u16,
    position_y: u16,
    offset_x: u16,
    offset_y: u16,
}

impl Editor {
    pub fn new() -> Self {
        let document = match std::env::args().nth(1) {
            Some(filename) => Document::open(&filename).unwrap_or_else(|_| Document::empty()),
            None => Document::empty(),
        };
        Self {
            should_quit: false,
            awaiting_g: false,
            document,
            mode: Mode::Normal,
            position_x: 0,
            position_y: 0,
            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;
            let event = read()?;
            self.dispatcher(event);

            // Exit
            if self.should_quit {
                break Ok(());
            }
        }
    }

    pub fn current_row_len(&self) -> u16 {
        self.document
            .rows
            .get(self.position_y as usize)
            .map_or(0, |line| line.len()) as u16
    }

    pub fn dispatcher(&mut self, event: Event) {
        let key = event.as_key_event().unwrap();
        match self.mode {
            Mode::Normal => self.handler_normal(key),
            Mode::Insert => self.handler_insert(key),
        }
    }

    pub fn handler_insert(&mut self, key: KeyEvent) {
        match key.code {
            // Switch mode to Normal
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                if self.position_x > 0 {
                    self.document
                        .delete_char(self.position_x - 1, self.position_y);
                    self.position_x -= 1;
                } else if self.position_y > 0 {
                    let prev_len = self.document.line_len(self.position_y - 1);
                    self.document.join_line(self.position_y);
                    self.position_y -= 1;
                    self.position_x = prev_len;
                }
            }
            KeyCode::Enter => {
                self.document
                    .insert_newline(self.position_x, self.position_y);
                self.position_y += 1;
                self.position_x = 0;
            }
            KeyCode::Char(c) => {
                self.document
                    .insert_char(self.position_x, self.position_y, c);
                self.position_x += 1;
            }
            _ => {}
        }
    }

    pub fn handler_normal(&mut self, key: KeyEvent) {
        let was_awaiting_g = self.awaiting_g;
        self.awaiting_g = false;

        match key.code {
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
                if (self.position_y as usize) + 1 < self.document.rows.len() {
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
                self.position_y = self.document.rows.len().saturating_sub(1) as u16;
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
                let max_y = self.document.rows.len() as u16;

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
            _ => {}
        }
    }


    pub fn refresh_screen(&mut self) -> Result<()> {
        self.scroll();
        let (cols, rows) = size()?;
        let mut out = stdout();

        queue!(out, Hide, MoveTo(0, 0))?; // Hide cursor, move to start

        // Document strings
        for i in 0..rows - 1 {
            queue!(out, Clear(ClearType::CurrentLine))?;
            let doc_row = self.offset_y as usize + i as usize;
            let content: String = match self.document.rows.get(doc_row) {
                Some(line) => line
                    .chars()
                    .skip(self.offset_x as usize)
                    .take(cols as usize)
                    .collect(),
                None => "~".to_string(),
            };
            write!(out, "{content}\r\n")?;
        }

        // Status line
        queue!(out, Clear(ClearType::CurrentLine))?;

        let name = self.document.filename.as_deref().unwrap_or("[No Name]");
        let modified = if self.document.dirty { " [+]" } else { "" };
        let status = format!(
            "{name}{modified} | {:?} | {}:{}",
            self.mode,
            self.position_y + 1,
            self.position_x + 1,
        );
        write!(out, "{status}")?;

        // Return cursor
        queue!(
            out,
            MoveTo(
                self.position_x - self.offset_x,
                self.position_y - self.offset_y
            ),
            Show
        )?;
        out.flush()?; // Apply changes
        Ok(())
    }

    pub fn scroll(&mut self) {
        let (cols, rows) = size().unwrap();
        let text_rows = rows - 1;
        // Vertical
        if self.position_y < self.offset_y {
            self.offset_y = self.position_y;
        } else if self.position_y >= self.offset_y + text_rows {
            self.offset_y = self.position_y - text_rows + 1;
        }
        // Horizontal
        if self.position_x < self.offset_x {
            self.offset_x = self.position_x;
        } else if self.position_x >= self.offset_x + cols {
            self.offset_x = self.position_x - cols + 1;
        }
    }

    pub fn clamp_x_to_row(&mut self) {
        let max_x = self.current_row_len().saturating_sub(1);
        if self.position_x > max_x {
            self.position_x = max_x;
        }
    }
}
