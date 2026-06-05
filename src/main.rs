use std::{
    self,
    fs::read_to_string,
    io::{stdout, Result},
};

use crossterm::{
    cursor::MoveTo,
    event::{read, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};

struct Editor {
    should_quit: bool,
    document: Document,
    position_x: u16,
    position_y: u16,
    offset_x: u16,
    offset_y: u16,
}

impl Editor {
    fn new() -> Self {
        let document = match std::env::args().nth(1) {
            Some(filename) => Document::open(&filename).unwrap_or_else(|_| Document::empty()),
            None => Document::empty(),
        };
        Self {
            should_quit: false,
            document,
            position_x: 0,
            position_y: 0,
            offset_x: 0,
            offset_y: 0,
        }
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;
            let event = read()?;
            self.handler_event(event);

            // Exit
            if self.should_quit {
                break Ok(());
            }
        }
    }

    fn current_row_len(&self) -> u16 {
        self.document
            .rows
            .get(self.position_y as usize)
            .map_or(0, |line| line.len()) as u16
    }

    fn handler_event(&mut self, event: Event) {
        let key = event.as_key_event().unwrap();
        match key.code {
            // Exit (ctrl+q)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            // Move left
            KeyCode::Char('h') => self.position_x = self.position_x.saturating_sub(1),
            // Move down
            KeyCode::Char('j') => {
                if (self.position_y as usize) + 1 < self.document.rows.len() {
                    self.position_y = self.position_y.saturating_add(1)
                }
                if self.current_row_len() <= self.position_x {
                    self.position_x = self.current_row_len().saturating_sub(1);
                }
            }
            // Move up
            KeyCode::Char('k') => {
                self.position_y = self.position_y.saturating_sub(1);
                if self.current_row_len() <= self.position_x {
                    self.position_x = self.current_row_len().saturating_sub(1);
                }
            }
            // Move right
            KeyCode::Char('l') if self.current_row_len() > self.position_x.saturating_add(1) => {
                self.position_x = self.position_x.saturating_add(1)
            }
            _ => {}
        }
    }

    fn set_cursor(&self, x: u16, y: u16) -> Result<()> {
        execute!(stdout(), MoveTo(x, y))?;
        Ok(())
    }

    fn clear_screen(&self) -> Result<()> {
        execute!(stdout(), Clear(ClearType::All))?;
        Ok(())
    }

    fn refresh_screen(&mut self) -> Result<()> {
        self.clear_screen()?;
        self.scroll();
        self.set_cursor(0, 0)?;
        let (cols, rows) = size()?;
        for i in 0..rows {
            let doc_row = self.offset_y as usize + i as usize;
            let content: String = match self.document.rows.get(doc_row) {
                Some(line) => line
                    .chars()
                    .skip(self.offset_x as usize)
                    .take(cols as usize)
                    .collect(),
                None => "~".to_string(),
            };
            if i < rows - 1 {
                print!("{content}\r\n");
            } else {
                print!("{content}");
            }
        }
        self.set_cursor(
            self.position_x - self.offset_x,
            self.position_y - self.offset_y,
        )?;
        Ok(())
    }

    fn scroll(&mut self) {
        let (cols, rows) = size().unwrap();
        // Vertical
        if self.position_y < self.offset_y {
            self.offset_y = self.position_y;
        } else if self.position_y >= self.offset_y + rows {
            self.offset_y = self.position_y - rows + 1;
        }
        // Horizontal
        if self.position_x < self.offset_x {
            self.offset_x = self.position_x;
        } else if self.position_x >= self.offset_x + cols {
            self.offset_x = self.position_x - cols + 1;
        }
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

struct Document {
    rows: Vec<String>,
}

impl Document {
    fn open(filename: &str) -> Result<Self> {
        let text = read_to_string(filename)?;
        let rows = text.lines().map(|line| line.to_string()).collect();
        Ok(Self { rows })
    }

    fn empty() -> Self {
        Self { rows: Vec::new() }
    }
}

fn main() -> Result<()> {
    let _guard = RawModeGuard::new()?;
    let mut editor = Editor::new();
    editor.run()
}
