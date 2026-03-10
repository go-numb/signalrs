use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignalError {
    #[error("TCP bind failed on {addr}: {source}")]
    TcpBind {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Mouse operation failed: {0}")]
    MouseOp(String),
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("Channel send failed")]
    ChannelClosed,
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
