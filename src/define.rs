use thiserror::Error;
use winsafe::OutputDebugString;

#[derive(Debug, Error)]
pub enum ErrorMess {
    #[error("establish connect fail, {0}")]
    WsaError(i32),

    #[error("convert fail string from bytes to u32")]
    ConvertError,

    #[error("status_handle_error, {0}")]
    StatusHandleError(String),

    #[error("error other")]
    otherErr,

    #[error("close accept")]
    ListenerClosed
}
pub fn dbg_print(log: &str) {
    OutputDebugString(log);
    #[cfg(debug_assertions)]
    println!("{}", log);
}

use windows_service::Error as WinServiceError;

impl From<WinServiceError> for ErrorMess {
    fn from(e: WinServiceError) -> Self {
        ErrorMess::StatusHandleError(format!("{e:?}"))
    }
}