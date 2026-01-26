// Test Server - Local HTTP server for integration tests
//
// Provides a local HTTP server serving test HTML pages.
// This enables deterministic, offline integration testing.

// Note: Functions appear "unused" because each test binary compiles separately,
// but they ARE used across multiple test files. Suppress false-positive warnings.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{Response, StatusCode},
    routing::get,
};
use std::net::SocketAddr;
use tokio::task::JoinHandle;

/// Test server handle
pub struct TestServer {
    addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestServer {
    /// Start the test server on a random available port
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/", get(index_page))
            .route("/button.html", get(button_page))
            .route("/form.html", get(form_page))
            .route("/input.html", get(input_page))
            .route("/dblclick.html", get(dblclick_page))
            .route("/keyboard.html", get(keyboard_page))
            .route("/locator.html", get(locator_page))
            .route("/locators.html", get(locators_page))
            .route("/checkbox.html", get(checkbox_page))
            .route("/hover.html", get(hover_page))
            .route("/select.html", get(select_page))
            .route("/upload.html", get(upload_page))
            .route("/keyboard_mouse.html", get(keyboard_mouse_page))
            .route("/click_options.html", get(click_options_page))
            .route("/text.html", get(text_page))
            .route("/websocket.html", get(websocket_page))
            .route("/anchors.html", get(anchors_page))
            .route("/ws", get(ws_handler));

        // Bind to port 0 to get any available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind test server");

        let addr = listener.local_addr().expect("Failed to get local address");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Test server failed");
        });

        TestServer { addr, handle }
    }

    /// Get the base URL of the test server
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Shutdown the test server
    pub fn shutdown(self) {
        self.handle.abort();
    }
}

// Test HTML pages

async fn index_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Test Index</title></head>
<body>
  <h1>Test Page</h1>
  <p>This is a test paragraph.</p>
  <a href="/button.html">Go to button page</a>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn button_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Button Test</title></head>
<body>
  <button id="btn" onclick="this.textContent='clicked'">Click me</button>
  <button id="btn2" onclick="this.textContent='clicked 2'">Click me 2</button>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn form_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Form Test</title></head>
<body>
  <form>
    <input type="text" id="name" name="name" />
    <textarea id="bio" name="bio"></textarea>
    <input type="submit" value="Submit" />
  </form>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn input_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Input Test</title></head>
<body>
  <input type="text" id="input" value="initial" />
  <input type="text" id="empty" value="" />
</body>
</html>"#,
        ))
        .unwrap()
}

async fn dblclick_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Double Click Test</title></head>
<body>
  <div id="target" ondblclick="this.textContent='double clicked'">Double click me</div>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn keyboard_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Keyboard Test</title></head>
<body>
  <input type="text" id="input" onkeydown="if(event.key==='Enter') this.value='submitted'" />
</body>
</html>"#,
        ))
        .unwrap()
}

async fn locator_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Locator Test</title></head>
<body>
  <h1>Test Page</h1>
  <p id="p1">First paragraph</p>
  <p id="p2">Second paragraph</p>
  <p id="p3">Third paragraph</p>
  <div class="container">
    <span id="nested">Nested element</span>
  </div>
  <div id="hidden" style="display: none;">Hidden element</div>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn locators_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Locators Test</title></head>
<body>
  <h1>Test Page</h1>
  <p id="p1">First paragraph</p>
  <p id="p2">Second paragraph</p>
  <p id="p3">Third paragraph</p>
  <p id="p4">Fourth paragraph</p>
  <div class="container">
    <span id="nested">Nested element</span>
  </div>
  <div id="hidden" style="display: none;">Hidden element</div>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn checkbox_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Checkbox Test</title></head>
<body>
  <input type="checkbox" id="checkbox" />
  <label for="checkbox">Unchecked checkbox</label>
  <br />
  <input type="checkbox" id="checked-checkbox" checked />
  <label for="checked-checkbox">Checked checkbox</label>
  <br />
  <input type="radio" id="radio1" name="radio-group" />
  <label for="radio1">Radio 1</label>
  <br />
  <input type="radio" id="radio2" name="radio-group" />
  <label for="radio2">Radio 2</label>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn hover_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head>
  <title>Hover Test</title>
  <style>
    #hover-button {
      padding: 10px;
      background-color: #ccc;
    }
    #tooltip {
      display: none;
      margin-top: 10px;
      padding: 5px;
      background-color: yellow;
    }
    #hover-button:hover + #tooltip {
      display: block;
    }
  </style>
</head>
<body>
  <button id="hover-button">Hover over me</button>
  <div id="tooltip">This is a tooltip</div>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn select_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Select Test</title></head>
<body>
  <select id="single-select">
    <option value="">--Please choose an option--</option>
    <option value="apple">Apple</option>
    <option value="banana">Banana</option>
    <option value="cherry">Cherry</option>
  </select>
  <br /><br />
  <select id="multi-select" multiple>
    <option value="red">Red</option>
    <option value="green">Green</option>
    <option value="blue">Blue</option>
    <option value="yellow">Yellow</option>
  </select>
  <br /><br />
  <select id="select-by-index">
    <option>First</option>
    <option>Second</option>
    <option>Third</option>
  </select>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn upload_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>File Upload Test</title></head>
<body>
  <input type="file" id="single-file" />
  <br /><br />
  <input type="file" id="multi-file" multiple />
  <br /><br />
  <div id="file-info"></div>
  <script>
    document.getElementById('single-file').addEventListener('change', (e) => {
      const files = Array.from(e.target.files).map(f => f.name).join(', ');
      document.getElementById('file-info').textContent = 'Single: ' + files;
    });
    document.getElementById('multi-file').addEventListener('change', (e) => {
      const files = Array.from(e.target.files).map(f => f.name).join(', ');
      document.getElementById('file-info').textContent = 'Multiple: ' + files;
    });
  </script>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn keyboard_mouse_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Keyboard and Mouse Test</title></head>
<body>
  <h1>Keyboard and Mouse Testing</h1>

  <input type="text" id="keyboard-input" placeholder="Type here" />
  <div id="keyboard-result"></div>

  <div id="clickable" style="width: 300px; height: 300px; background-color: lightblue; margin-top: 20px;">
    Click or double-click me
  </div>
  <div id="mouse-result"></div>
  <div id="mouse-coords"></div>

  <script>
    // Keyboard event handlers
    document.getElementById('keyboard-input').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        document.getElementById('keyboard-result').textContent = 'Enter pressed';
      }
    });

    // Mouse event handlers
    document.getElementById('clickable').addEventListener('click', (e) => {
      document.getElementById('mouse-result').textContent = 'Clicked';
    });

    document.getElementById('clickable').addEventListener('dblclick', (e) => {
      document.getElementById('mouse-result').textContent = 'Double-clicked';
    });

    document.addEventListener('mousemove', (e) => {
      document.getElementById('mouse-coords').textContent = `Mouse: (${e.clientX}, ${e.clientY})`;
    });
  </script>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn click_options_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Click Options Test</title></head>
<body>
  <button id="button">Click Me</button>
  <button id="hidden-button" style="display: none;">Hidden Button</button>
  <div id="result"></div>
  <script>
    const button = document.getElementById('button');
    const hiddenButton = document.getElementById('hidden-button');
    const result = document.getElementById('result');

    // Track all mouse events
    button.addEventListener('mousedown', (e) => {
      const buttonName = e.button === 0 ? 'left' : e.button === 1 ? 'middle' : 'right';
      result.textContent = `mousedown button:${buttonName} shiftKey:${e.shiftKey} ctrlKey:${e.ctrlKey}`;
    });

    button.addEventListener('click', (e) => {
      const buttonName = e.button === 0 ? 'left' : e.button === 1 ? 'middle' : 'right';
      result.textContent = `click button:${buttonName} shiftKey:${e.shiftKey} ctrlKey:${e.ctrlKey}`;
    });

    button.addEventListener('contextmenu', (e) => {
      e.preventDefault(); // Prevent context menu
      result.textContent = `contextmenu (right) shiftKey:${e.shiftKey} ctrlKey:${e.ctrlKey}`;
    });

    button.addEventListener('auxclick', (e) => {
      const buttonName = e.button === 1 ? 'middle' : e.button === 2 ? 'right' : 'other';
      result.textContent = `auxclick button:${buttonName} shiftKey:${e.shiftKey} ctrlKey:${e.ctrlKey}`;
    });

    button.addEventListener('dblclick', (e) => {
      result.textContent = 'dblclick';
    });

    hiddenButton.addEventListener('click', (e) => {
      result.textContent = 'hidden-button-clicked';
    });
  </script>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn text_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Text Assertions Test</title></head>
<body>
  <h1>Welcome to Playwright</h1>
  <p id="whitespace">
    Text with whitespace
  </p>
  <p id="long-text">This is the beginning and middle of the text and the end.</p>
  <input type="text" id="name-input" value="John Doe" />
  <input type="text" id="empty-input" value="" />
</body>
</html>"#,
        ))
        .unwrap()
}

async fn websocket_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            r#"<!DOCTYPE html>
<html>
<head><title>WebSocket Test</title></head>
<body>
  <h1>WebSocket Test</h1>
  <div id="log"></div>
  <script>
    const log = document.getElementById('log');
    const ws = new WebSocket('ws://' + location.host + '/ws');

    ws.onopen = () => {
        log.textContent += 'open\n';
        ws.send('Hello Server');
    };

    ws.onmessage = (event) => {
        log.textContent += 'received: ' + event.data + '\n';
    };

    ws.onclose = () => {
        log.textContent += 'closed\n';
    };
  </script>
</body>
</html>"#,
        ))
        .unwrap()
}

async fn anchors_page() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(
            "<!DOCTYPE html>
<html>
<head><title>Anchor Navigation Test</title></head>
<body>
  <h1>Anchor Navigation Test Page</h1>

  <nav>
    <a id=\"link-to-section1\" href=\"#section1\">Go to Section 1</a> |
    <a id=\"link-to-section2\" href=\"#section2\">Go to Section 2</a> |
    <a id=\"link-to-section3\" href=\"#section3\">Go to Section 3</a>
  </nav>

  <section id=\"section1\" style=\"margin-top: 50px; padding: 20px; background: #f0f0f0;\">
    <h2>Section 1</h2>
    <p>This is section 1. The URL should include #section1 when you navigate here.</p>
  </section>

  <section id=\"section2\" style=\"margin-top: 50px; padding: 20px; background: #e0e0e0;\">
    <h2>Section 2</h2>
    <p>This is section 2. The URL should include #section2 when you navigate here.</p>
  </section>

  <section id=\"section3\" style=\"margin-top: 50px; padding: 20px; background: #d0d0d0;\">
    <h2>Section 3</h2>
    <p>This is section 3. The URL should include #section3 when you navigate here.</p>
  </section>
</body>
</html>",
        ))
        .unwrap()
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                // Echo back
                let _ = socket.send(Message::Text(text)).await;
            }
        } else {
            // Client likely disconnected
            return;
        }
    }
}
