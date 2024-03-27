use std::fmt::{self, Debug};

use cursive::{reexports::log::error, view::Nameable, views::Dialog, Cursive};

/// The error type.
#[repr(i64)]
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// This File already exists
    AlreadyExists,
    /// The user provided arguments are malformed
    Arguments,
    /// A file could not be found, opened or saved
    FileOpen,
    /// The Text could not be saved to the clipboard
    Clipboard,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyExists => write!(f, "This filepath already exists. Saving the file now will overwrite the data of the existing file, you've been warned!\nCheck the file path and change it accordingly."),
            Error::Arguments => write!(f, "Ensure the provided arguments are correctly formatted.\nForce quit via ctrl + f or toggle the debugger via ctrl + d"),
            Error::FileOpen => write!(f, "The requested file could not be found, opened, or saved. Check the file path and permissions.\nForce quit via ctrl + f or toggle the debugger via ctrl + d"),
            Error::Clipboard => write!(f, "Failed to save/get text to/from the clipboard. Ensure your clipboard manager is running.\nForce quit via ctrl + f or toggle the debugger via ctrl + d"),
        }
    }
}

impl From<clippers::Error> for Error {
    fn from(value: clippers::Error) -> Self {
        error!("clippers::Error: {value:?}");
        Self::Clipboard
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(e: std::convert::Infallible) -> Self {
        error!("convert::Infallible: {e:?}");
        Self::Arguments
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        error!("File Error: {e}");
        Self::FileOpen
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
                .button("OK", |s| {
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
