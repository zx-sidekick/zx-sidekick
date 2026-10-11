//! What can go wrong with the machine.

use std::fmt;

/// An error from the machine or a snapshot.
#[derive(Debug)]
pub enum Error {
    /// The emulator core reported an error.
    Core(rustzx_core::error::Error),
    /// A snapshot names a model this program doesn't emulate.
    UnknownModel,
    /// A snapshot is for the other model.
    OtherModel,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Core(_) => "the emulated machine",
            Self::UnknownModel => "the snapshot is for a machine other than a 48K or 128K",
            Self::OtherModel => "the snapshot is for the other model (48K or 128K)",
        })
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(e) => Some(e),
            Self::UnknownModel | Self::OtherModel => None,
        }
    }
}

impl From<rustzx_core::error::Error> for Error {
    fn from(error: rustzx_core::error::Error) -> Self {
        Self::Core(error)
    }
}
