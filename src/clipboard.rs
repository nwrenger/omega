use arboard::Clipboard;

use crate::error::Result;

/// Gets the clipboard content (text)
pub fn get_content() -> Result<String> {
    let mut clipboard = Clipboard::new()?;

    Ok(clipboard.get_text()?)
}

/// Sets the clipboard content
pub fn set_content(content: String) -> Result<()> {
    let mut clipboard = Clipboard::new()?;

    Ok(clipboard.set_text(content)?)
}
