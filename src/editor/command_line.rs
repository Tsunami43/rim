/// A parsed `:` command.
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Command {
    Save,
    Quit,
    SaveQuit,
    Unknown,
}

/// The text typed after `:` in command mode.
pub struct CommandLine {
    buffer: String,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Whether nothing has been typed yet.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Append a typed character.
    pub fn push(&mut self, c: char) {
        self.buffer.push(c);
    }

    // pub fn as_str(&self) -> &str {
    //     &self.buffer
    // }

    /// Remove the last character (Backspace).
    pub fn pop(&mut self) {
        self.buffer.pop();
    }

    /// Reset the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Interpret the typed text as a command.
    pub fn parse(&self) -> Command {
        match self.buffer.as_str() {
            "w" => Command::Save,
            "q" => Command::Quit,
            "wq" => Command::SaveQuit,
            "qw" => Command::SaveQuit,
            _ => Command::Unknown,
        }
    }
}
