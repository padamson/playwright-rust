// playwright-core: Internal implementation of Playwright protocol for Rust
//
// This crate is not part of the public API and should only be used by the
// `playwright` crate.

pub mod api;
pub mod channel;
pub mod channel_owner;
pub mod connection;
pub mod driver;
pub mod error;
pub mod object_factory;
pub mod protocol;
pub mod server;
pub mod transport;

pub use api::{IgnoreDefaultArgs, LaunchOptions, ProxySettings};
pub use channel::Channel;
pub use channel_owner::{ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection};
pub use connection::{Connection, ConnectionLike};
pub use error::{Error, Result};
pub use protocol::{BrowserType, Playwright};
pub use server::PlaywrightServer;
pub use transport::{PipeTransport, PipeTransportReceiver, Transport};
