// Copyright 2026 Paul Adamson
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

pub mod accessibility;
pub mod action_options;
pub mod android;
pub mod api_request_context;
pub mod artifact;
pub mod binding_call;
pub mod browser;
pub mod browser_context;
pub mod browser_type;
pub mod cdp_session;
pub mod click;
pub mod clock;
pub mod console_message;
pub mod coverage;
pub mod device;
pub mod dialog;
pub mod download;
pub mod drag_to;
pub mod electron;
pub mod element_handle;
pub mod evaluate_conversion;
pub mod event_value;
pub mod event_waiter;
pub mod file_chooser;
pub mod file_payload;
pub mod frame;
pub mod frame_locator;
pub mod js_handle;
pub mod keyboard;
pub mod local_utils;
pub mod locator;
pub mod mouse;
pub mod page;
pub mod playwright;
pub mod proxy;
pub mod request;
pub mod response;
pub mod root;
pub mod route;
pub mod screenshot;
pub mod select_option;
pub mod selectors;
pub mod tap;
pub mod touchscreen;
pub mod tracing;
pub mod video;
pub mod wait_for;
pub mod web_error;
pub mod web_socket;
pub mod web_socket_route;
pub mod worker;

pub use accessibility::{Accessibility, AccessibilitySnapshotOptions};
pub use action_options::{
    CheckOptions, FillOptions, HoverOptions, KeyboardOptions, MouseOptions, PressOptions,
    PressSequentiallyOptions, SelectOptions,
};
pub use android::Android;
pub use api_request_context::{
    APIRequest, APIRequestContext, APIRequestContextOptions, APIResponse,
};
pub use binding_call::BindingCall;
pub use browser::{Browser, StartTracingOptions};
pub use browser_context::{
    AcceptDownloads, BrowserContext, BrowserContextOptions, BrowserContextOptionsBuilder,
    ClearCookiesOptions, Cookie, Geolocation, GrantPermissionsOptions, LocalStorageItem, Origin,
    RecordHar, RecordVideo, StorageState, Viewport,
};
pub use browser_type::BrowserType;
pub use cdp_session::CDPSession;
pub use click::{ClickOptions, KeyboardModifier, MouseButton, Position};
pub use clock::{Clock, ClockInstallOptions};
pub use console_message::{ConsoleMessage, ConsoleMessageLocation};
pub use coverage::{
    CSSCoverageEntry, Coverage, CoverageRange, JSCoverageEntry, JSCoverageRange,
    JSFunctionCoverage, StartCSSCoverageOptions, StartJSCoverageOptions,
};
pub use device::{DeviceDescriptor, DeviceViewport};
pub use dialog::Dialog;
pub use download::Download;
pub use drag_to::{DragToOptions, DragToOptionsBuilder};
pub use electron::Electron;
pub use element_handle::ElementHandle;
pub use evaluate_conversion::{parse_result, parse_value, serialize_argument, serialize_null};
pub use event_value::EventValue;
pub use event_waiter::EventWaiter;
pub use file_chooser::FileChooser;
pub use file_payload::{FilePayload, FilePayloadBuilder};
pub use frame::Frame;
pub use frame_locator::FrameLocator;
pub use js_handle::JSHandle;
pub use keyboard::Keyboard;
pub use local_utils::LocalUtils;
pub use locator::{AriaRole, BoundingBox, FilterOptions, GetByRoleOptions, Locator};
pub use mouse::Mouse;
pub use page::{
    AddLocatorHandlerOptions, AddScriptTagOptions, AddScriptTagOptionsBuilder, AddStyleTagOptions,
    ColorScheme, EmulateMediaOptions, EmulateMediaOptionsBuilder, ForcedColors, GotoOptions, Media,
    Page, PdfMargin, PdfOptions, PdfOptionsBuilder, ReducedMotion, Response, RouteFromHarOptions,
    WaitUntil,
};
pub use playwright::Playwright;
pub use proxy::ProxySettings;
pub use request::{Request, ResourceTiming};
pub use response::{HeaderEntry, RemoteAddr, RequestSizes, ResponseObject, SecurityDetails};
pub use root::Root;
pub use route::{
    ContinueOptions, ContinueOptionsBuilder, FetchOptions, FetchOptionsBuilder, FetchResponse,
    FulfillOptions, FulfillOptionsBuilder, Route, UnrouteBehavior,
};
pub use screenshot::{ScreenshotClip, ScreenshotOptions, ScreenshotType};
pub use select_option::SelectOption;
pub use selectors::Selectors;
pub use tap::{TapOptions, TapOptionsBuilder};
pub use touchscreen::Touchscreen;
pub use tracing::{Tracing, TracingStartOptions, TracingStopOptions};
pub use video::Video;
pub use wait_for::{WaitForOptions, WaitForOptionsBuilder, WaitForState};
pub use web_error::WebError;
pub use web_socket::WebSocket;
pub use web_socket_route::{WebSocketRoute, WebSocketRouteCloseOptions};
pub use worker::Worker;
