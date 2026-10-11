//! What can go wrong with the machine.

/// An error from the machine or a snapshot.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The emulator core reported an error.
    #[error("the emulated machine: {0}")]
    Core(rustzx_core::error::Error),
    /// A snapshot names a model this program doesn't emulate.
    #[error("the snapshot is for a machine other than a 48K or 128K")]
    UnknownModel,
    /// A snapshot is for the other model.
    #[error("the snapshot is for the other model (48K or 128K)")]
    OtherModel,
}

impl From<rustzx_core::error::Error> for Error {
    fn from(error: rustzx_core::error::Error) -> Self {
        Self::Core(error)
    }
}
