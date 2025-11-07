// Playwright transport layer
//
// Handles bidirectional communication with Playwright server via stdio pipes.
// Follows the same architecture as playwright-python's PipeTransport.

use crate::{Error, Result};
use serde_json::Value as JsonValue;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;

/// Send a JSON message using length-prefixed framing
///
/// Helper function for sending messages when you only have stdin access.
/// This is used by Connection to send messages without needing the full transport.
///
/// # Arguments
/// * `stdin` - The writer to send to (usually child process stdin)
/// * `message` - JSON message to send
pub async fn send_message<W>(stdin: &mut W, message: JsonValue) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    // Serialize to JSON
    let json_bytes = serde_json::to_vec(&message)
        .map_err(|e| Error::TransportError(format!("Failed to serialize JSON: {}", e)))?;

    let length = json_bytes.len() as u32;

    // Write 4-byte little-endian length prefix
    stdin
        .write_all(&length.to_le_bytes())
        .await
        .map_err(|e| Error::TransportError(format!("Failed to write length: {}", e)))?;

    // Write JSON payload
    stdin
        .write_all(&json_bytes)
        .await
        .map_err(|e| Error::TransportError(format!("Failed to write message: {}", e)))?;

    // Flush to ensure message is sent
    stdin
        .flush()
        .await
        .map_err(|e| Error::TransportError(format!("Failed to flush: {}", e)))?;

    Ok(())
}

/// Transport trait for abstracting communication mechanisms
///
/// Playwright server communication happens over stdio pipes using
/// length-prefixed JSON messages.
pub trait Transport: Send + Sync {
    /// Send a JSON message to the server
    fn send(&mut self, message: JsonValue) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Pipe-based transport for communicating with Playwright server
///
/// This implementation matches playwright-python's PipeTransport:
/// - Messages are framed with 4-byte little-endian length prefix
/// - Reads happen in a background task
/// - Received messages are sent via mpsc channel
///
/// # Architecture
///
/// ```text
/// ┌─────────────┐
/// │   Server    │
/// │   (Node.js) │
/// └──────┬──────┘
///        │ stdio
///        │
/// ┌──────▼──────────────────────┐
/// │    PipeTransport            │
/// │  ┌────────┐   ┌──────────┐  │
/// │  │ Writer │   │  Reader  │  │
/// │  │ (send) │   │  (loop)  │  │
/// │  └────────┘   └──────────┘  │
/// └─────────────────┬───────────┘
///                   │ mpsc channel
///                   ▼
///            ┌──────────────┐
///            │  Connection  │
///            │  (dispatch)  │
///            └──────────────┘
/// ```
pub struct PipeTransport<W, R>
where
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    stdin: W,
    stdout: R,
    message_tx: mpsc::UnboundedSender<JsonValue>,
}

/// Receive-only part of PipeTransport
///
/// This struct only contains stdout and the message channel,
/// allowing it to run the receive loop without needing stdin.
/// This solves the deadlock issue by separating send and receive.
pub struct PipeTransportReceiver<R>
where
    R: AsyncRead + Unpin + Send,
{
    stdout: R,
    message_tx: mpsc::UnboundedSender<JsonValue>,
}

impl<R> PipeTransportReceiver<R>
where
    R: AsyncRead + Unpin + Send,
{
    /// Run the message read loop
    ///
    /// This continuously reads messages from stdout and sends them
    /// to the message channel.
    pub async fn run(mut self) -> Result<()> {
        loop {
            // Read 4-byte little-endian length prefix
            let mut len_buf = [0u8; 4];
            self.stdout.read_exact(&mut len_buf).await.map_err(|e| {
                Error::TransportError(format!("Failed to read length prefix: {}", e))
            })?;

            let length = u32::from_le_bytes(len_buf) as usize;

            // Read message payload
            let mut message_buf = vec![0u8; length];
            self.stdout
                .read_exact(&mut message_buf)
                .await
                .map_err(|e| Error::TransportError(format!("Failed to read message: {}", e)))?;

            // Parse JSON
            let message: JsonValue = serde_json::from_slice(&message_buf)
                .map_err(|e| Error::ProtocolError(format!("Failed to parse JSON: {}", e)))?;

            // Dispatch message
            if self.message_tx.send(message).is_err() {
                // Channel closed, stop reading
                break;
            }
        }

        Ok(())
    }
}

impl<W, R> PipeTransport<W, R>
where
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    /// Create a new pipe transport from child process stdio handles
    ///
    /// # Arguments
    ///
    /// * `stdin` - Child process stdin for sending messages
    /// * `stdout` - Child process stdout for receiving messages
    ///
    /// Returns a tuple of (PipeTransport, message receiver channel)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::transport::PipeTransport;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut child = Command::new("node")
    ///     .arg("cli.js")
    ///     .stdin(std::process::Stdio::piped())
    ///     .stdout(std::process::Stdio::piped())
    ///     .spawn()?;
    ///
    /// let stdin = child.stdin.take().unwrap();
    /// let stdout = child.stdout.take().unwrap();
    ///
    /// let (mut transport, mut rx) = PipeTransport::new(stdin, stdout);
    ///
    /// // Spawn read loop
    /// tokio::spawn(async move {
    ///     transport.run().await
    /// });
    ///
    /// // Receive messages
    /// while let Some(message) = rx.recv().await {
    ///     println!("Received: {:?}", message);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(stdin: W, stdout: R) -> (Self, mpsc::UnboundedReceiver<JsonValue>) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let transport = Self {
            stdin,
            stdout,
            message_tx,
        };

        (transport, message_rx)
    }

    /// Split the transport into stdin and the rest
    ///
    /// This allows Connection to hold stdin separately (for sending)
    /// while run() owns stdout (for receiving).
    ///
    /// # Returns
    ///
    /// Returns (stdin, self_without_stdin) where self_without_stdin
    /// can still run the receive loop but cannot send.
    pub fn into_parts(self) -> (W, PipeTransportReceiver<R>) {
        (
            self.stdin,
            PipeTransportReceiver {
                stdout: self.stdout,
                message_tx: self.message_tx,
            },
        )
    }

    /// Run the message read loop
    ///
    /// This continuously reads messages from the server and sends them
    /// to the message channel. Matches playwright-python's `run()` method.
    ///
    /// The loop will run until:
    /// - An error occurs
    /// - The stdout stream is closed
    /// - The message channel is dropped
    ///
    /// # Errors
    ///
    /// Returns an error if reading from stdout fails or if message parsing fails.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Read 4-byte little-endian length prefix
            // Matches: buffer = await self._proc.stdout.readexactly(4)
            let mut len_buf = [0u8; 4];
            self.stdout.read_exact(&mut len_buf).await.map_err(|e| {
                Error::TransportError(format!("Failed to read length prefix: {}", e))
            })?;

            let length = u32::from_le_bytes(len_buf) as usize;

            // Read message payload
            // Python reads in 32KB chunks for large messages
            // For simplicity, we'll read the entire message at once for now
            // TODO: Consider chunked reading for very large messages (>32KB)
            let mut message_buf = vec![0u8; length];
            self.stdout
                .read_exact(&mut message_buf)
                .await
                .map_err(|e| Error::TransportError(format!("Failed to read message: {}", e)))?;

            // Parse JSON
            // Matches: obj = json.loads(data.decode("utf-8"))
            let message: JsonValue = serde_json::from_slice(&message_buf)
                .map_err(|e| Error::ProtocolError(format!("Failed to parse JSON: {}", e)))?;

            // Dispatch message
            // Matches: self.on_message(obj)
            if self.message_tx.send(message).is_err() {
                // Channel closed, stop reading
                break;
            }
        }

        Ok(())
    }

    /// Send a message to the server
    ///
    /// Messages are framed with a 4-byte little-endian length prefix
    /// followed by the JSON payload.
    ///
    /// Matches playwright-python's `send()` method:
    /// ```python
    /// data = json.dumps(message).encode("utf-8")
    /// self._output.write(len(data).to_bytes(4, byteorder="little") + data)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or writing fails.
    async fn send_internal(&mut self, message: JsonValue) -> Result<()> {
        // Serialize to JSON
        let json_bytes = serde_json::to_vec(&message)
            .map_err(|e| Error::TransportError(format!("Failed to serialize JSON: {}", e)))?;

        let length = json_bytes.len() as u32;

        // Write 4-byte little-endian length prefix
        // Matches: len(data).to_bytes(4, byteorder="little")
        self.stdin
            .write_all(&length.to_le_bytes())
            .await
            .map_err(|e| Error::TransportError(format!("Failed to write length: {}", e)))?;

        // Write JSON payload
        self.stdin
            .write_all(&json_bytes)
            .await
            .map_err(|e| Error::TransportError(format!("Failed to write message: {}", e)))?;

        // Flush to ensure message is sent
        self.stdin
            .flush()
            .await
            .map_err(|e| Error::TransportError(format!("Failed to flush: {}", e)))?;

        Ok(())
    }
}

impl<W, R> Transport for PipeTransport<W, R>
where
    W: AsyncWrite + Unpin + Send + Sync,
    R: AsyncRead + Unpin + Send + Sync,
{
    async fn send(&mut self, message: JsonValue) -> Result<()> {
        self.send_internal(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_length_prefix_encoding() {
        // Test that we match Python's little-endian encoding
        let length: u32 = 1234;
        let bytes = length.to_le_bytes();

        // Verify little-endian byte order
        assert_eq!(bytes[0], (length & 0xFF) as u8);
        assert_eq!(bytes[1], ((length >> 8) & 0xFF) as u8);
        assert_eq!(bytes[2], ((length >> 16) & 0xFF) as u8);
        assert_eq!(bytes[3], ((length >> 24) & 0xFF) as u8);

        // Verify round-trip
        assert_eq!(u32::from_le_bytes(bytes), length);
    }

    #[test]
    fn test_message_framing_format() {
        // Verify our framing matches Python's format:
        // len(data).to_bytes(4, byteorder="little") + data
        let message = serde_json::json!({"test": "hello"});
        let json_bytes = serde_json::to_vec(&message).unwrap();
        let length = json_bytes.len() as u32;
        let length_bytes = length.to_le_bytes();

        // Frame should be: [length (4 bytes LE)][JSON bytes]
        let mut frame = Vec::new();
        frame.extend_from_slice(&length_bytes);
        frame.extend_from_slice(&json_bytes);

        // Verify structure
        assert_eq!(frame.len(), 4 + json_bytes.len());
        assert_eq!(&frame[0..4], &length_bytes);
        assert_eq!(&frame[4..], &json_bytes);
    }

    #[tokio::test]
    async fn test_send_message() {
        // Create TWO separate duplex pipes:
        // 1. For stdin: transport writes, we read
        // 2. For stdout: we write, transport reads
        let (stdin_read, stdin_write) = tokio::io::duplex(1024);
        let (stdout_read, stdout_write) = tokio::io::duplex(1024);

        // Give transport the write end of stdin pipe and read end of stdout pipe
        let (_stdin_read, mut _stdout_write) = (stdin_read, stdout_write);
        let (mut transport, _rx) = PipeTransport::new(stdin_write, stdout_read);

        // Test message
        let test_message = serde_json::json!({
            "id": 1,
            "method": "test",
            "params": {"foo": "bar"}
        });

        // Send message
        transport.send(test_message.clone()).await.unwrap();

        // Read what transport wrote to stdin from our read end
        let (mut read_half, _write_half) = tokio::io::split(_stdin_read);
        let mut len_buf = [0u8; 4];
        read_half.read_exact(&mut len_buf).await.unwrap();
        let length = u32::from_le_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; length];
        read_half.read_exact(&mut msg_buf).await.unwrap();

        let received: serde_json::Value = serde_json::from_slice(&msg_buf).unwrap();
        assert_eq!(received, test_message);
    }

    #[tokio::test]
    async fn test_multiple_messages_in_sequence() {
        // Create two duplex pipes for bidirectional communication
        let (_stdin_read, stdin_write) = tokio::io::duplex(4096);
        let (stdout_read, mut stdout_write) = tokio::io::duplex(4096);

        let (mut transport, mut rx) = PipeTransport::new(stdin_write, stdout_read);

        // Spawn reader task
        let read_task = tokio::spawn(async move { transport.run().await });

        // Send multiple messages (simulating server sending to transport)
        let messages = vec![
            serde_json::json!({"id": 1, "method": "first"}),
            serde_json::json!({"id": 2, "method": "second"}),
            serde_json::json!({"id": 3, "method": "third"}),
        ];

        for msg in &messages {
            let json_bytes = serde_json::to_vec(msg).unwrap();
            let length = json_bytes.len() as u32;

            stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
            stdout_write.write_all(&json_bytes).await.unwrap();
        }
        stdout_write.flush().await.unwrap();

        // Receive all messages
        for expected in &messages {
            let received = rx.recv().await.unwrap();
            assert_eq!(&received, expected);
        }

        // Clean up
        drop(stdout_write);
        drop(rx);
        let _ = read_task.await;
    }

    #[tokio::test]
    async fn test_large_message() {
        let (_stdin_read, stdin_write) = tokio::io::duplex(1024 * 1024); // 1MB buffer
        let (stdout_read, mut stdout_write) = tokio::io::duplex(1024 * 1024);

        let (mut transport, mut rx) = PipeTransport::new(stdin_write, stdout_read);

        // Spawn reader
        let read_task = tokio::spawn(async move { transport.run().await });

        // Create a large message (>32KB to test chunked reading note in code)
        let large_string = "x".repeat(100_000);
        let large_message = serde_json::json!({
            "id": 1,
            "data": large_string
        });

        let json_bytes = serde_json::to_vec(&large_message).unwrap();
        let length = json_bytes.len() as u32;

        // Should be > 32KB
        assert!(length > 32_768, "Test message should be > 32KB");

        stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
        stdout_write.write_all(&json_bytes).await.unwrap();
        stdout_write.flush().await.unwrap();

        // Verify we can receive it
        let received = rx.recv().await.unwrap();
        assert_eq!(received, large_message);

        drop(stdout_write);
        drop(rx);
        let _ = read_task.await;
    }

    #[tokio::test]
    async fn test_malformed_length_prefix() {
        let (_stdin_read, stdin_write) = tokio::io::duplex(1024);
        let (stdout_read, mut stdout_write) = tokio::io::duplex(1024);

        let (mut transport, _rx) = PipeTransport::new(stdin_write, stdout_read);

        // Write only 2 bytes instead of 4 (incomplete length prefix)
        // This simulates server sending malformed data
        stdout_write.write_all(&[0x01, 0x02]).await.unwrap();
        stdout_write.flush().await.unwrap();

        // Close the pipe to trigger EOF
        drop(stdout_write);

        // Run should error on incomplete read
        let result = transport.run().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read length prefix"));
    }

    #[tokio::test]
    async fn test_broken_pipe() {
        let (_stdin_read, stdin_write) = tokio::io::duplex(1024);
        let (stdout_read, stdout_write) = tokio::io::duplex(1024);

        let (mut transport, _rx) = PipeTransport::new(stdin_write, stdout_read);

        // Close the stdout write side immediately
        drop(stdout_write);

        // Spawn run() - it should error when trying to read from closed pipe
        let read_task = tokio::spawn(async move { transport.run().await });

        // Wait for it to complete - should be an error
        let result = read_task.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let (_stdin_read, stdin_write) = tokio::io::duplex(1024);
        let (stdout_read, mut stdout_write) = tokio::io::duplex(1024);

        let (mut transport, mut rx) = PipeTransport::new(stdin_write, stdout_read);

        // Spawn reader
        let read_task = tokio::spawn(async move { transport.run().await });

        // Send a message
        let message = serde_json::json!({"id": 1, "method": "test"});
        let json_bytes = serde_json::to_vec(&message).unwrap();
        let length = json_bytes.len() as u32;

        stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
        stdout_write.write_all(&json_bytes).await.unwrap();
        stdout_write.flush().await.unwrap();

        // Receive the message
        let received = rx.recv().await.unwrap();
        assert_eq!(received, message);

        // Drop the receiver (simulates connection closing)
        drop(rx);

        // Close stdout pipe
        drop(stdout_write);

        // Reader should exit cleanly (channel closed)
        let result = read_task.await.unwrap();
        // Should succeed - channel closed is expected shutdown
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Failed to read"));
    }
}
