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
mod assertions;
mod browser;
mod browser_context;
mod checkbox;
mod click_options;
mod connection;
mod context_route;
mod context_runtime_setters;
mod downloads_dialogs;
mod element_handle;
mod evaluate;
mod initialization;
mod keyboard_mouse;
mod launch_context;
mod locator;
mod navigation;
mod network_route;
mod page;
mod page_event_network;
mod page_new_methods;
mod pause;
mod playwright_launch;
mod route_advanced;
mod route_fallback_unroute;
mod route_fetch;
mod screenshot;
mod scripts_styles;
mod select_upload;
mod stability;
mod storage_state;
mod transport;
mod windows_cleanup;
