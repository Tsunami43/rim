mod action;
mod command_line;
mod input;
mod keymap;
mod render;
use std::{self, io::Result};

use crate::{document::Document, editor::command_line::CommandLine};

use crossterm::event::{Event, read};

use keymap::Keymap;

/// Current editing mode.
#[derive(Clone, Debug, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

/// A pending operator waiting for a motion target (e.g. `d` in `dw`).
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Operator {
    Delete,
}

/// The editor: owns the document, cursor/viewport state, mode and keymap.
pub struct Editor {
    should_quit: bool,
    awaiting_g: bool,
    awaiting_replace: bool,
    pending_op: Option<Operator>,
    document: Document,
    mode: Mode,
    position_x: u16,
    position_y: u16,
    offset_x: u16,
    offset_y: u16,
    command_line: CommandLine,
    keymap: Keymap,
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
            awaiting_replace: false,
            pending_op: None,
            document,
            mode: Mode::Normal,
            position_x: 0,
            position_y: 0,
            offset_x: 0,
            offset_y: 0,
            command_line: CommandLine::new(),
            keymap: Keymap::default_vim(),
        }
    }

    /// Main loop: draw, read one event, dispatch it, repeat until quit.
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

    /// Length of the line the cursor is on.
    pub fn current_row_len(&self) -> u16 {
        self.document.line_len(self.position_y as usize) as u16
    }

    /// Route a key event to the handler for the current mode.
    pub fn dispatcher(&mut self, event: Event) {
        let key = event.as_key_event().unwrap();
        match self.mode {
            Mode::Normal => self.handler_normal(key),
            Mode::Insert => self.handler_insert(key),
            Mode::Command => self.handler_command(key),
        }
    }

    /// Keep the cursor column within the current line.
    pub fn clamp_x_to_row(&mut self) {
        let max_x = self.current_row_len().saturating_sub(1);
        if self.position_x > max_x {
            self.position_x = max_x;
        }
    }

    /// Keep the cursor row within the document.
    fn clamp_y_to_doc(&mut self) {
        let last = self.document.rows_len().saturating_sub(1) as u16;
        if self.position_y > last {
            self.position_y = last;
        }
    }
}
