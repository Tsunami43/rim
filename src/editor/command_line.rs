#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Command {
    Save,
    Quit,
    SaveQuit,
    Unknown,
}

pub struct CommandLine {
    buffer: String,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn push(&mut self, c: char) {
        self.buffer.push(c);
    }

    // pub fn as_str(&self) -> &str {
    //     &self.buffer
    // }

    pub fn pop(&mut self) {
        self.buffer.pop();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

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
