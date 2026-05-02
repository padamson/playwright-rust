// Single integration test binary
//
// All async integration tests are consolidated into this binary to reduce
// compilation time (1 binary instead of 53). Nextest still runs each
// #[tokio::test] function in parallel.
//
// Sync-only tests (no browser needed) remain as separate binaries in tests/.

mod common;
mod test_server;

mod actions;
mod api_request;
mod aria_snapshot;
mod assertions;
mod back_references;
mod browser;
mod browser_context;
mod cdp_tracing;
mod checkbox;
mod click_options;
mod connection;
mod console_message;
mod context_events;
mod context_route;
mod context_runtime_setters;
mod debugger;
mod downloads_dialogs;
mod element_handle;
mod evaluate;
mod expect_event;
mod expose_binding;
mod file_chooser;
mod frame_api;
mod frame_locator;
mod initialization;
mod install_browsers;
mod js_handle;
mod keyboard_mouse;
mod launch_context;
mod locator;
mod navigation;
mod network_route;
mod page;
mod page_assertions;
mod page_event_network;
mod page_events;
mod page_new_methods;
mod page_properties;
mod pause;
mod playwright_launch;
mod request;
mod request_response_complete;
mod response;
mod route_advanced;
mod route_fallback_unroute;
mod route_fetch;
mod screenshot;
mod scripts_styles;
mod select_upload;
mod selectors;
mod stability;
mod storage_state;
mod transport;
mod web_error;
mod websocket;
mod windows_cleanup;
mod worker;
