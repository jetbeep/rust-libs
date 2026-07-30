use alloc::string::String;

#[cfg(feature = "platform-zephyr")]
use alloc::{str, string::ToString};

#[derive(Clone, Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Error({}): {}", self.code, self.message)
    }
}

#[cfg(feature = "platform-desktop")]
impl std::error::Error for Error {}

#[cfg(feature = "platform-zephyr")]
pub fn from_jb_error(error: *const crate::c_bindings::jb_error_t) -> Error {
    unsafe {
        Error {
            code: (*error).code,
            message: str::from_utf8_unchecked((*error).str_.as_ref()).to_string(),
        }
    }
}
