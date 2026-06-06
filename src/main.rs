mod document;
mod editor;
use crate::editor::Editor;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::{self, io::Result};
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
