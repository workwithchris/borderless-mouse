pub mod protocol;
pub mod transport;
pub mod config;
pub mod input;
pub mod network;

pub use protocol::*;
pub use transport::*;
pub use config::*;
pub use input::*;
pub use network::*;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
