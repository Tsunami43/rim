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
    Visual,
    Search,
}

/// A pending operator waiting for a motion target (e.g. `d` in `dw`).
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Operator {
    Delete,
    Change,
    Yank,
}

/// The yank/delete register (a tiny clipboard).
#[derive(Clone, Debug, PartialEq)]
pub enum Register {
    None,
    /// Characterwise text (e.g. from `yw`, `x`).
    Char(String),
    /// Linewise text (e.g. from `yy`, `dd`).
    Line(String),
}

/// The editor: owns the document, cursor/viewport state, mode and keymap.
pub struct Editor {
    should_quit: bool,
    awaiting_g: bool,
    awaiting_replace: bool,
    /// Numeric prefix being accumulated in Normal mode (e.g. `3` in `3j`).
    count: Option<usize>,
    pending_op: Option<Operator>,
    document: Document,
    mode: Mode,
    position_x: u16,
    position_y: u16,
    offset_x: u16,
    offset_y: u16,
    /// Visual-mode selection anchor (the fixed end of the selection).
    anchor: Option<(u16, u16)>,
    /// Yank/delete register.
    register: Register,
    /// History of document snapshots for undo/redo.
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    /// Last search pattern (reused by `n`/`N`/`*`).
    last_search: String,
    command_line: CommandLine,
    keymap: Keymap,
}

/// A saved document state plus cursor, for undo/redo.
struct Snapshot {
    document: Document,
    position_x: u16,
    position_y: u16,
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
            count: None,
            pending_op: None,
            document,
            mode: Mode::Normal,
            position_x: 0,
            position_y: 0,
            offset_x: 0,
            offset_y: 0,
            anchor: None,
            register: Register::None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_search: String::new(),
            command_line: CommandLine::new(),
            keymap: Keymap::default_vim(),
        }
    }

    /// Jump to the next match of the last search pattern (`n`).
    pub fn search_next(&mut self) {
        if self.last_search.is_empty() {
            return;
        }
        let from = (self.position_x as usize, self.position_y as usize);
        if let Some((x, y)) = self.document.find(&self.last_search, from) {
            self.position_x = x as u16;
            self.position_y = y as u16;
            self.clamp_x_to_row();
        }
    }

    /// Jump to the previous match of the last search pattern (`N`).
    pub fn search_prev(&mut self) {
        if self.last_search.is_empty() {
            return;
        }
        let from = (self.position_x as usize, self.position_y as usize);
        if let Some((x, y)) = self.document.rfind(&self.last_search, from) {
            self.position_x = x as u16;
            self.position_y = y as u16;
            self.clamp_x_to_row();
        }
    }

    /// Search for the word under the cursor (`*`).
    pub fn search_word(&mut self) {
        if let Some(word) = self
            .document
            .word_at(self.position_x as usize, self.position_y as usize)
        {
            self.last_search = word;
            self.search_next();
        }
    }

    /// Save the current state for undo, and clear the redo history.
    /// Call this right before a mutating change.
    pub fn push_undo(&mut self) {
        self.undo_stack.push(Snapshot {
            document: self.document.clone(),
            position_x: self.position_x,
            position_y: self.position_y,
        });
        self.redo_stack.clear();
    }

    /// Restore the previous state (`u`).
    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(Snapshot {
                document: self.document.clone(),
                position_x: self.position_x,
                position_y: self.position_y,
            });
            self.document = prev.document;
            self.position_x = prev.position_x;
            self.position_y = prev.position_y;
            self.clamp_y_to_doc();
            self.clamp_x_to_row();
        }
    }

    /// Re-apply an undone state (`Ctrl-r`).
    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(Snapshot {
                document: self.document.clone(),
                position_x: self.position_x,
                position_y: self.position_y,
            });
            self.document = next.document;
            self.position_x = next.position_x;
            self.position_y = next.position_y;
            self.clamp_y_to_doc();
            self.clamp_x_to_row();
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
            Mode::Visual => self.handler_visual(key),
            Mode::Search => self.handler_search(key),
        }
    }

    /// Ordered selection bounds `(start, end)` while in Visual mode, inclusive.
    fn selection_bounds(&self) -> Option<((u16, u16), (u16, u16))> {
        if self.mode != Mode::Visual {
            return None;
        }
        let anchor = self.anchor?;
        let cursor = (self.position_x, self.position_y);
        if (anchor.1, anchor.0) <= (cursor.1, cursor.0) {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    /// Whether cell `(x, y)` is inside the current Visual selection.
    pub fn is_selected(&self, x: u16, y: u16) -> bool {
        match self.selection_bounds() {
            Some((start, end)) => (start.1, start.0) <= (y, x) && (y, x) <= (end.1, end.0),
            None => false,
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
