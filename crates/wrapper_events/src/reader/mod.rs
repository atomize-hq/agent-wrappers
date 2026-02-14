mod sync;

#[cfg(feature = "tokio")]
mod tokio;

pub use sync::{BoundedLine, SyncBoundedLineReader};

#[cfg(feature = "tokio")]
pub use tokio::{AsyncBoundedLineReader, AsyncBoundedLineResult};
