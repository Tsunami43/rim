use std::{self, io::Result};

use crossterm::{
    event::{read, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

struct Editor {
    should_quit: bool,
}

impl Editor {
    fn new() -> Self {
        Self { should_quit: false }
    }
    fn run(&mut self) -> Result<()> {
        loop {
            let event = read()?;
            let key = event.as_key_event().unwrap();
            println!("Press button(code): {:?}", key.code);
            if key.code == KeyCode::Esc {
                self.should_quit = true;
            }
            if self.should_quit {
                break Ok(());
            }
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

fn main() -> Result<()> {
    let _guard = RawModeGuard::new()?;
    let mut editor = Editor::new();
    editor.run()
}
