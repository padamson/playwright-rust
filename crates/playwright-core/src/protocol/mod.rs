// Copyright 2024 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Protocol Objects - Rust representations of Playwright protocol objects
//
// This module contains the Rust implementations of all Playwright protocol objects.
// Each object corresponds to a type in the Playwright protocol (protocol.yml).
//
// Architecture:
// - All protocol objects implement the ChannelOwner trait
// - Objects are created by the object factory when server sends __create__ messages
// - Objects communicate with the server via their Channel

pub mod browser;
pub mod browser_context;
pub mod browser_type;
pub mod frame;
pub mod page;
pub mod playwright;
pub mod request;
pub mod response;
pub mod root;

pub use browser::Browser;
pub use browser_context::BrowserContext;
pub use browser_type::BrowserType;
pub use frame::Frame;
pub use page::{GotoOptions, Page, Response, WaitUntil};
pub use playwright::Playwright;
pub use request::Request;
pub use response::ResponseObject;
pub use root::Root;
