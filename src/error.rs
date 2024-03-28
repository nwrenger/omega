use std::fmt::{self, Debug};

use cursive::{reexports::log::error, view::Nameable, views::Dialog, Cursive};

/// The error type.
#[repr(i64)]
#[derive(Debug, Clone)]
pub enum Error {
    /// The user provided arguments are malformed
    Arguments(String),
    /// A file could not be found, opened or saved
    FileOpen(String),
    /// The Text could not be saved to the clipboard
    Clipboard(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Arguments(e) => write!(f, "Arguments: {e}.\nForce quit via ctrl + f or toggle the goto via ctrl + d"),
            Error::FileOpen(e) => write!(f, "File System Error: {e}. Check the file path and permissions.\nForce quit via ctrl + f or toggle the goto via ctrl + o"),
            Error::Clipboard(e) => write!(f, "Clipboard: {e}. Ensure your clipboard manager is running.\nForce quit via ctrl + f or toggle the goto via ctrl + d"),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(e: std::convert::Infallible) -> Self {
        error!("convert::Infallible: {e}");
        Self::Arguments(e.to_string())
    }
}

impl From<clippers::Error> for Error {
    fn from(e: clippers::Error) -> Self {
        error!("clippers::Error: {e}");
        Self::Clipboard(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        error!("File Error: {e}");
        Self::FileOpen(e.to_string())
    }
}

impl Error {
    /// Converts this error into a UI element for a Cursive application.
    pub fn to_dialog(self, siv: &mut Cursive) {
        if let Some(pos) = siv.screen_mut().find_layer_from_name("error") {
            siv.screen_mut().remove_layer(pos);
        }
        let error_message = self.to_string();
        siv.add_layer(
            Dialog::text(error_message)
                .title("Error")
                .padding_lrtb(1, 1, 1, 0)
                .button("Ok", |s| {
                    s.pop_layer();
                })
                .with_name("error"),
        );
    }
}

/// Result type using the api error.
pub type Result<T> = std::result::Result<T, Error>;

/// Extension for handler function
pub trait ResultExt<T> {
    fn handle(self, siv: &mut Cursive);
}

impl<T> ResultExt<T> for Result<T> {
    /// Result Handler for showing in the UI
    fn handle(self, siv: &mut Cursive) {
        if let Err(e) = self {
            e.to_dialog(siv);
        }
    }
}
