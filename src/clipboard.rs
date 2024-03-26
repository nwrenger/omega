use crate::error::Result;

/// Gets the clipboard content (text)
pub fn get_content() -> Result<String> {
    use crate::error::Error;

    let mut clipboard = clippers::Clipboard::get();

    if let Some(clippers::ClipperData::Text(text)) = clipboard.read() {
        Ok(text.to_string())
    } else {
        Err(Error::Clipboard)
    }
}

/// Sets the clipboard content
pub fn set_content(content: String) -> Result<()> {
    let mut clipboard = clippers::Clipboard::get();

    Ok(clipboard.write_text(content)?)
}
