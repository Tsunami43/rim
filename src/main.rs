use std::{
    self,
    fs::read_to_string,
    io::{stdout, Result},
};

use crossterm::{
    cursor::MoveTo,
    event::{read, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};

struct Editor {
    should_quit: bool,
    document: Document,
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
        }
    }
    fn run(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;
            let event = read()?;
            let key = event.as_key_event().unwrap();
            if key.code == KeyCode::Esc {
                self.should_quit = true;
            }
            if self.should_quit {
                break Ok(());
            }
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

    fn refresh_screen(&self) -> Result<()> {
        self.clear_screen()?;
        self.set_cursor(0, 0)?;
        let (_, rows) = size()?;
        for i in 0..rows {
            match self.document.rows.get(i as usize) {
                Some(line) => print!("{line}\r\n"),
                None => print!("~\r\n"),
            }
        }
        self.set_cursor(0, 0)?;
        Ok(())
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
