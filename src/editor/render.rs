use super::Editor;
use std::{
    self,
    io::{Result, Write, stdout},
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    queue,
    terminal::{Clear, ClearType, size},
};

impl Editor {
    pub fn refresh_screen(&mut self) -> Result<()> {
        self.scroll();
        let (cols, rows) = size()?;
        let mut out = stdout();

        queue!(out, Hide, MoveTo(0, 0))?; // Hide cursor, move to start

        // Document strings
        for i in 0..rows - 1 {
            queue!(out, Clear(ClearType::CurrentLine))?;
            let doc_row = self.offset_y as usize + i as usize;
            let content: String = match self.document.row(doc_row) {
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

        let name = self.document.filename().unwrap_or("[No Name]");
        let modified = if self.document.is_dirty() { " [+]" } else { "" };
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
}
