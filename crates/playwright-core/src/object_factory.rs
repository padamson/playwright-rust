// Copyright 2024 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Object Factory - Creates protocol objects from type names
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_connection.py (_create_remote_object)
// - Java: playwright-java/.../impl/Connection.java (createRemoteObject)
// - JavaScript: playwright/.../client/connection.ts (_createRemoteObject)
//
// The object factory maps protocol type names (strings) to Rust constructors.
// When the server sends a `__create__` message, the factory instantiates
// the appropriate Rust object based on the type name.

use crate::channel_owner::{ChannelOwner, ParentOrConnection};
use crate::error::{Error, Result};
use crate::protocol::{
    Browser, BrowserContext, BrowserType, Frame, Page, Playwright, Request, ResponseObject,
};
use serde_json::Value;
use std::sync::Arc;

/// Creates a protocol object from a `__create__` message.
///
/// This function is the central object factory for the Playwright protocol.
/// It maps type names from the server to Rust struct constructors.
///
/// # Arguments
///
/// * `parent` - Either a parent ChannelOwner or the root Connection
/// * `type_name` - Protocol type name (e.g., "Playwright", "BrowserType")
/// * `guid` - Unique GUID assigned by the server
/// * `initializer` - JSON object with initial state
///
/// # Returns
///
/// An `Arc<dyn ChannelOwner>` pointing to the newly created object.
///
/// # Errors
///
/// Returns `Error::ProtocolError` if the type name is unknown or if
/// object construction fails.
///
/// # Example
///
/// ```no_run
/// # use playwright_core::object_factory::create_object;
/// # use playwright_core::channel_owner::ParentOrConnection;
/// # use playwright_core::connection::ConnectionLike;
/// # use std::sync::Arc;
/// # use serde_json::json;
/// # async fn example(connection: Arc<dyn ConnectionLike>) -> Result<(), Box<dyn std::error::Error>> {
/// let playwright_obj = create_object(
///     ParentOrConnection::Connection(connection),
///     "Playwright".to_string(),
///     "playwright@1".to_string(),
///     json!({
///         "chromium": { "guid": "browserType@chromium" },
///         "firefox": { "guid": "browserType@firefox" },
///         "webkit": { "guid": "browserType@webkit" }
///     })
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_object(
    parent: ParentOrConnection,
    type_name: String,
    guid: String,
    initializer: Value,
) -> Result<Arc<dyn ChannelOwner>> {
    // Match on type name and call appropriate constructor
    let object: Arc<dyn ChannelOwner> = match type_name.as_str() {
        "Playwright" => {
            // Playwright is the root object, so parent must be Connection
            let connection = match parent {
                ParentOrConnection::Connection(conn) => conn,
                ParentOrConnection::Parent(_) => {
                    return Err(Error::ProtocolError(
                        "Playwright must have Connection as parent".to_string(),
                    ))
                }
            };

            Arc::new(Playwright::new(connection, type_name, guid, initializer).await?)
        }

        "BrowserType" => {
            // BrowserType has Playwright as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "BrowserType must have Playwright as parent".to_string(),
                    ))
                }
            };

            Arc::new(BrowserType::new(
                parent_owner,
                type_name,
                guid,
                initializer,
            )?)
        }

        "Browser" => {
            // Browser has BrowserType as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Browser must have BrowserType as parent".to_string(),
                    ))
                }
            };

            Arc::new(Browser::new(parent_owner, type_name, guid, initializer)?)
        }

        "BrowserContext" => {
            // BrowserContext has Browser as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "BrowserContext must have Browser as parent".to_string(),
                    ))
                }
            };

            Arc::new(BrowserContext::new(
                parent_owner,
                type_name,
                guid,
                initializer,
            )?)
        }

        "Page" => {
            // Page has BrowserContext as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Page must have BrowserContext as parent".to_string(),
                    ))
                }
            };

            Arc::new(Page::new(parent_owner, type_name, guid, initializer)?)
        }

        "Frame" => {
            // Frame has Page as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Frame must have Page as parent".to_string(),
                    ))
                }
            };

            Arc::new(Frame::new(parent_owner, type_name, guid, initializer)?)
        }

        "Request" => {
            // Request has Frame as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Request must have Frame as parent".to_string(),
                    ))
                }
            };

            Arc::new(Request::new(parent_owner, type_name, guid, initializer)?)
        }

        "Response" => {
            // Response has Request as parent (not Frame!)
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Response must have Request as parent".to_string(),
                    ))
                }
            };

            Arc::new(ResponseObject::new(
                parent_owner,
                type_name,
                guid,
                initializer,
            )?)
        }

        // TODO: Add more types as they are implemented in future phases:
        // "ElementHandle" => Arc::new(ElementHandle::new(parent_owner, type_name, guid, initializer)?),
        // ... etc
        _ => {
            // Unknown type - log warning and return error
            tracing::warn!("Unknown protocol type: {}", type_name);
            return Err(Error::ProtocolError(format!(
                "Unknown protocol type: {}",
                type_name
            )));
        }
    };

    Ok(object)
}

// Note: Object factory testing is done via integration tests since it requires:
// - Real Connection with object registry
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
