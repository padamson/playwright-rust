// Copyright 2026 Paul Adamson
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

use crate::error::{Error, Result};
use crate::protocol::{
    APIRequestContext, Android, Browser, BrowserContext, BrowserType, Dialog, Electron, Frame,
    LocalUtils, Page, Playwright, Request, ResponseObject, Route, Tracing, WebSocket,
    artifact::Artifact,
};
use crate::server::channel_owner::{ChannelOwner, ParentOrConnection};
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
/// ```ignore
/// # use playwright_rs::server::object_factory::create_object;
/// # use playwright_rs::server::channel_owner::ParentOrConnection;
/// # use playwright_rs::server::connection::ConnectionLike;
/// # use std::sync::Arc;
/// # use serde_json::json;
/// # async fn example(connection: Arc<dyn ConnectionLike>) -> Result<(), Box<dyn std::error::Error>> {
/// let playwright_obj = create_object(
///     ParentOrConnection::Connection(connection),
///     "Playwright".to_string(),
///     Arc::from("playwright@1"),
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
    guid: Arc<str>,
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
                    ));
                }
            };

            Arc::new(Playwright::new(connection, type_name, guid, initializer).await?)
        }

        "BrowserType" => {
            // BrowserType is a root child (created with parent="")
            // The Playwright object references them via its initializer
            Arc::new(BrowserType::new(parent, type_name, guid, initializer)?)
        }

        "Browser" => {
            // Browser has BrowserType as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Browser must have BrowserType as parent".to_string(),
                    ));
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
                    ));
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
                    ));
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
                    ));
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
                    ));
                }
            };

            Arc::new(Request::new(parent_owner, type_name, guid, initializer)?)
        }

        "Route" => {
            // Route has Frame as parent (created during network interception)
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Route must have Frame as parent".to_string(),
                    ));
                }
            };

            Arc::new(Route::new(parent_owner, type_name, guid, initializer)?)
        }

        "Response" => {
            // Response has Request as parent (not Frame!)
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Response must have Request as parent".to_string(),
                    ));
                }
            };

            Arc::new(ResponseObject::new(
                parent_owner,
                type_name,
                guid,
                initializer,
            )?)
        }

        "ElementHandle" => {
            // ElementHandle has Frame as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "ElementHandle must have Frame as parent".to_string(),
                    ));
                }
            };

            Arc::new(crate::protocol::ElementHandle::new(
                parent_owner,
                type_name,
                guid,
                initializer,
            )?)
        }

        "Artifact" => {
            // Artifact has BrowserContext as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Artifact must have BrowserContext as parent".to_string(),
                    ));
                }
            };

            Arc::new(Artifact::new(parent_owner, type_name, guid, initializer)?)
        }

        "Dialog" => {
            // Dialog has Page as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "Dialog must have Page as parent".to_string(),
                    ));
                }
            };

            Arc::new(Dialog::new(parent_owner, type_name, guid, initializer)?)
        }

        "Android" => {
            // Android stub
            Arc::new(Android::new(parent, type_name, guid, initializer)?)
        }

        "Electron" => {
            // Electron stub
            Arc::new(Electron::new(parent, type_name, guid, initializer)?)
        }

        "Tracing" => {
            // Tracing stub
            Arc::new(Tracing::new(parent, type_name, guid, initializer)?)
        }

        "APIRequestContext" => {
            // APIRequestContext stub
            Arc::new(APIRequestContext::new(
                parent,
                type_name,
                guid,
                initializer,
            )?)
        }

        "LocalUtils" => {
            // LocalUtils stub
            Arc::new(LocalUtils::new(parent, type_name, guid, initializer)?)
        }

        "WebSocket" => {
            // WebSocket has Page as parent
            let parent_owner = match parent {
                ParentOrConnection::Parent(p) => p,
                ParentOrConnection::Connection(_) => {
                    return Err(Error::ProtocolError(
                        "WebSocket must have Page as parent".to_string(),
                    ));
                }
            };

            Arc::new(WebSocket::new(parent_owner, type_name, guid, initializer)?)
        }

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
