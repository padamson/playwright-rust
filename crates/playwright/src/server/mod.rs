//! Server management and connection layer (internal)
//!
//! This module handles the Playwright server lifecycle, JSON-RPC communication,
//! and channel-based object management.
//!
//! **Note**: This module is exposed publicly only for integration testing purposes.
//! The types and APIs in this module are considered internal implementation details
//! and may change without notice. User code should not depend on these types directly.

#[doc(hidden)]
pub mod channel;
#[doc(hidden)]
pub mod channel_owner;
#[doc(hidden)]
pub mod connection;
#[doc(hidden)]
pub mod driver;
#[doc(hidden)]
pub mod object_factory;
#[doc(hidden)]
pub mod playwright_server;
#[doc(hidden)]
pub mod transport;
