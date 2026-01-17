use super::{Transport, TransportReceiver, TransportSender};
use crate::{Error, Result};
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;

/// Send a JSON message using length-prefixed framing
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

/// Pipe-based transport for communicating with Playwright server
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
    pub async fn run_loop(&mut self) -> Result<()> {
        const CHUNK_SIZE: usize = 32_768; // 32KB chunks

        loop {
            // Read 4-byte little-endian length prefix
            let mut len_buf = [0u8; 4];

            let n = self.stdout.read(&mut len_buf).await.map_err(|e| {
                Error::TransportError(format!("Failed to read length prefix: {}", e))
            })?;

            if n == 0 {
                break;
            }

            if n < 4 {
                self.stdout
                    .read_exact(&mut len_buf[n..])
                    .await
                    .map_err(|e| {
                        Error::TransportError(format!(
                            "Failed to finish reading length prefix: {}",
                            e
                        ))
                    })?;
            }

            let length = u32::from_le_bytes(len_buf) as usize;

            // Read message payload
            let message_buf = if length <= CHUNK_SIZE {
                let mut buf = vec![0u8; length];
                self.stdout
                    .read_exact(&mut buf)
                    .await
                    .map_err(|e| Error::TransportError(format!("Failed to read message: {}", e)))?;
                buf
            } else {
                let mut buf = Vec::with_capacity(length);
                let mut remaining = length;

                while remaining > 0 {
                    let to_read = std::cmp::min(remaining, CHUNK_SIZE);
                    let mut chunk = vec![0u8; to_read];

                    self.stdout.read_exact(&mut chunk).await.map_err(|e| {
                        Error::TransportError(format!("Failed to read message chunk: {}", e))
                    })?;

                    buf.extend_from_slice(&chunk);
                    remaining -= to_read;
                }

                buf
            };

            let message: JsonValue = serde_json::from_slice(&message_buf)
                .map_err(|e| Error::ProtocolError(format!("Failed to parse JSON: {}", e)))?;

            if self.message_tx.send(message).is_err() {
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
    pub fn new(stdin: W, stdout: R) -> (Self, mpsc::UnboundedReceiver<JsonValue>) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let transport = Self {
            stdin,
            stdout,
            message_tx,
        };

        (transport, message_rx)
    }

    pub fn into_parts(self) -> (W, PipeTransportReceiver<R>) {
        (
            self.stdin,
            PipeTransportReceiver {
                stdout: self.stdout,
                message_tx: self.message_tx,
            },
        )
    }
}

// Implement Transport trait in the same module where struct is defined
impl<W, R> Transport for PipeTransport<W, R>
where
    W: AsyncWrite + Unpin + Send + Sync,
    R: AsyncRead + Unpin + Send + Sync,
{
    async fn send(&mut self, message: JsonValue) -> Result<()> {
        send_message(&mut self.stdin, message).await
    }
}

impl<W> TransportSender for W
where
    W: AsyncWrite + Unpin + Send,
{
    fn send(
        &mut self,
        message: JsonValue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move { send_message(self, message).await })
    }
}

impl<R> TransportReceiver for PipeTransportReceiver<R>
where
    R: AsyncRead + Unpin + Send,
{
    fn run(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move { self.run_loop().await })
    }
}
