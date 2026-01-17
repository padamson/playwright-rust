// Public API types module
//
// This module contains high-level API types that are used across the protocol layer.
// These types provide builder patterns and ergonomic interfaces for protocol operations.

pub mod connect_options;
pub mod launch_options;

pub use connect_options::ConnectOptions;
pub use launch_options::{IgnoreDefaultArgs, LaunchOptions, ProxySettings};
