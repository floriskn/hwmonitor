use std::fmt;
use windows::core::Error as WinError;

#[derive(Debug)]
pub enum AffinityError {
    /// WinAPI returned an unexpected error
    WinApi {
        context: &'static str,
        source: WinError,
    },

    /// Failed to query thread affinity
    GetThreadAffinityFailed,

    /// Failed to set thread affinity
    SetThreadAffinityFailed,

    /// Worker thread panicked
    ThreadPanic,
}

impl fmt::Display for AffinityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WinApi { context, source } => {
                write!(f, "{}: {:?}", context, source)
            }
            Self::GetThreadAffinityFailed => {
                write!(f, "Failed to get thread group affinity")
            }
            Self::SetThreadAffinityFailed => {
                write!(f, "Failed to set thread group affinity")
            }
            Self::ThreadPanic => write!(f, "Worker thread panicked"),
        }
    }
}

impl std::error::Error for AffinityError {}
