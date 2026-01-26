use super::{Transport, TransportReceiver, TransportSender};
use crate::{Error, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use url::Url;

/// WebSocket transport for remote browser connections
pub struct WebSocketTransport {
    message_tx: mpsc::UnboundedSender<JsonValue>,

    // Let's store the sender half of the split stream
    sender: futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,

    // Readers are handled by splitting in into_parts()
    receiver: Option<futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

pub struct WebSocketTransportReceiver {
    receiver: futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    message_tx: mpsc::UnboundedSender<JsonValue>,
}

impl WebSocketTransport {
    pub async fn connect(
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<JsonValue>)> {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        // Parse URL to ensure validity
        let _parsed_url =
            Url::parse(url).map_err(|e| Error::TransportError(format!("Invalid URL: {}", e)))?;

        // Create base request from URL string (adds Sec-WebSocket-Key, etc.)
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = url
            .into_client_request()
            .map_err(|e| Error::TransportError(format!("Failed to build request: {}", e)))?;

        // Add custom headers
        if let Some(headers_map) = headers {
            use std::str::FromStr;
            use tokio_tungstenite::tungstenite::http::header::{HeaderName, HeaderValue};
            let headers = request.headers_mut();
            for (k, v) in headers_map {
                let name = HeaderName::from_str(&k)
                    .map_err(|e| Error::TransportError(format!("Invalid header name: {}", e)))?;
                let value = HeaderValue::from_str(&v)
                    .map_err(|e| Error::TransportError(format!("Invalid header value: {}", e)))?;
                headers.insert(name, value);
            }
        }

        // Connect
        let (ws_stream, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| Error::TransportError(format!("WebSocket connection failed: {}", e)))?;

        let (sender, receiver) = ws_stream.split();

        Ok((
            Self {
                message_tx,
                sender,
                receiver: Some(receiver),
            },
            message_rx,
        ))
    }

    pub fn into_parts(mut self) -> (WebSocketTransportSender, WebSocketTransportReceiver) {
        let receiver = self.receiver.take().expect("Receiver already taken");

        let sender = WebSocketTransportSender {
            sender: self.sender,
        };

        let receiver = WebSocketTransportReceiver {
            receiver,
            message_tx: self.message_tx,
        };

        (sender, receiver)
    }
}

// Wrapper for the sender part
pub struct WebSocketTransportSender {
    sender: futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
}

impl TransportSender for WebSocketTransportSender {
    fn send(
        &mut self,
        message: JsonValue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let json_str = serde_json::to_string(&message)
                .map_err(|e| Error::TransportError(format!("Failed to serialize JSON: {}", e)))?;

            self.sender
                .send(WsMessage::Text(json_str.into()))
                .await
                .map_err(|e| {
                    Error::TransportError(format!("Failed to send WebSocket message: {}", e))
                })
        })
    }
}

impl TransportReceiver for WebSocketTransportReceiver {
    fn run(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            while let Some(msg_result) = self.receiver.next().await {
                match msg_result {
                    Ok(msg) => {
                        match msg {
                            WsMessage::Text(text) => {
                                let message: JsonValue =
                                    serde_json::from_str(&text).map_err(|e| {
                                        Error::ProtocolError(format!("Failed to parse JSON: {}", e))
                                    })?;

                                if self.message_tx.send(message).is_err() {
                                    break;
                                }
                            }
                            WsMessage::Binary(_) => {
                                // Playwright usually uses Text for JSON RPC
                                // But might use binary for Buffer transfer?
                                // For now ignore or log
                            }
                            WsMessage::Close(_) => break,
                            _ => {}
                        }
                    }
                    Err(e) => {
                        return Err(Error::TransportError(format!(
                            "WebSocket read error: {}",
                            e
                        )));
                    }
                }
            }
            Ok(())
        })
    }
}

impl Transport for WebSocketTransport {
    async fn send(&mut self, message: JsonValue) -> Result<()> {
        let json_str = serde_json::to_string(&message)
            .map_err(|e| Error::TransportError(format!("Failed to serialize JSON: {}", e)))?;

        self.sender
            .send(WsMessage::Text(json_str.into()))
            .await
            .map_err(|e| Error::TransportError(format!("Failed to send WebSocket message: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing_in_connect() {
        // We can't easily test connect() because it tries to make a real connection.
        // Integration tests will handle the actual connection.
        // But we can verify Url crate behavior if we want.
        let url = Url::parse("ws://127.0.0.1:9000").unwrap();
        assert_eq!(url.port(), Some(9000));
    }
}
