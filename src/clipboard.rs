use clippers::{Clipboard, ClipperData};

use crate::error::{Error, Result};

/// Gets the clipboard content (text)
pub fn get_content() -> Result<String> {
    let mut clipboard = Clipboard::get();

    if let Some(ClipperData::Text(text)) = clipboard.read() {
        Ok(text.to_string())
    } else {
        Err(Error::Clipboard)
    }
}

/// Sets the clipboard content
pub fn set_content(content: String) -> Result<()> {
    let mut clipboard = Clipboard::get();

    Ok(clipboard.write_text(content)?)
}
