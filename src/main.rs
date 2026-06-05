use std::{self, io::Result};

use crossterm::{
    event::{read, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
fn main() -> Result<()> {
    enable_raw_mode()?;
    loop {
        let event = read()?;
        let key = event.as_key_event().unwrap();
        println!("Press button(code): {:?}", key.code);
        if key.code == KeyCode::Esc {
            break;
        }
    }
    disable_raw_mode()?;
    Ok(())
}
