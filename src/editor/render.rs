use super::{Editor, Mode};
use std::{
    self,
    io::{Result, Write, stdout},
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    queue,
    style::{Attribute, SetAttribute},
    terminal::{Clear, ClearType, size},
};

impl Editor {
    /// Redraw the whole frame: document rows, status line, cursor.
    /// Everything is queued into one buffer and flushed once to avoid flicker.
    pub fn refresh_screen(&mut self) -> Result<()> {
        self.scroll();
        let (cols, rows) = size()?;
        let mut out = stdout();

        queue!(out, Hide, MoveTo(0, 0))?; // Hide cursor, move to start

        // Document rows (highlighting the Visual selection cell by cell)
        for i in 0..rows - 1 {
            queue!(out, Clear(ClearType::CurrentLine))?;
            let doc_row = self.offset_y as usize + i as usize;
            match self.document.row(doc_row) {
                Some(line) => {
                    let visible = line
                        .chars()
                        .skip(self.offset_x as usize)
                        .take(cols as usize);
                    for (col, ch) in (self.offset_x as usize..).zip(visible) {
                        if self.is_selected(col as u16, doc_row as u16) {
                            queue!(out, SetAttribute(Attribute::Reverse))?;
                            write!(out, "{ch}")?;
                            queue!(out, SetAttribute(Attribute::Reset))?;
                        } else {
                            write!(out, "{ch}")?;
                        }
                    }
                    write!(out, "\r\n")?;
                }
                None => write!(out, "~\r\n")?,
            }
        }

        // Bottom line: the `:`/`/` input prompt, otherwise the status line
        let prompt = match self.mode {
            Mode::Command => Some(':'),
            Mode::Search => Some('/'),
            _ => None,
        };
        queue!(out, Clear(ClearType::CurrentLine))?;
        if let Some(p) = prompt {
            write!(out, "{p}{}", self.command_line.as_str())?;
        } else {
            let name = self.document.filename().unwrap_or("[No Name]");
            let modified = if self.document.is_dirty() { " [+]" } else { "" };
            let status = format!(
                "{name}{modified} | {:?} | {}:{}",
                self.mode,
                self.position_y + 1,
                self.position_x + 1,
            );
            write!(out, "{status}")?;
        }

        // Place the cursor: on the prompt line while typing a command/search,
        // otherwise back in the text
        if prompt.is_some() {
            let col = self.command_line.as_str().chars().count() as u16 + 1;
            queue!(out, MoveTo(col, rows - 1), Show)?;
        } else {
            queue!(
                out,
                MoveTo(
                    self.position_x - self.offset_x,
                    self.position_y - self.offset_y
                ),
                Show
            )?;
        }
        out.flush()?; // Apply changes
        Ok(())
    }

    /// Adjust the viewport offset so the cursor stays on screen.
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
