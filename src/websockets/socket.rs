use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Simple WebSocket abstraction - all we care about is send/receive
#[async_trait]
pub trait SocketWrapper: Send {
    /// Send a text message to the client
    async fn send_message(&mut self, message: String) -> Result<(), SocketError>;

    /// Receive the next message from the client (None if connection closed)
    async fn receive_message(&mut self) -> Result<Option<String>, SocketError>;

    /// Close the connection
    async fn close(&mut self) -> Result<(), SocketError>;
}

/// Handler for incoming WebSocket messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message from the client
    async fn handle_message(&self, username: &str, room_id: &str, message: String);
}

#[derive(Debug)]
pub enum SocketError {
    ConnectionClosed,
    SendFailed(String),
    ReceiveFailed(String),
}

/// Direct implementation on axum's WebSocket
#[async_trait]
impl SocketWrapper for WebSocket {
    async fn send_message(&mut self, message: String) -> Result<(), SocketError> {
        self.send(Message::Text(message))
            .await
            .map_err(|e| SocketError::SendFailed(e.to_string()))
    }

    async fn receive_message(&mut self) -> Result<Option<String>, SocketError> {
        match self.next().await {
            Some(Ok(Message::Text(text))) => Ok(Some(text)),
            Some(Ok(Message::Close(_))) => Ok(None),
            Some(Ok(_)) => Ok(None), // Ignore binary/ping/pong
            Some(Err(e)) => Err(SocketError::ReceiveFailed(e.to_string())),
            None => Ok(None), // Connection closed
        }
    }

    async fn close(&mut self) -> Result<(), SocketError> {
        self.send(Message::Close(None))
            .await
            .map_err(|e| SocketError::SendFailed(e.to_string()))
    }
}

/// Connection represents a managed WebSocket connection
/// It is used to send and receive messages to and from the client
/// The outbound receiver is a channel that receives messages from the ConnectionManager's outbound sender
pub struct Connection {
    pub username: String,
    pub room_id: String,
    socket: Box<dyn SocketWrapper>,
    outbound_receiver: mpsc::UnboundedReceiver<String>,
    message_handler: Arc<dyn MessageHandler>,
}

impl Connection {
    pub fn new(
        username: String,
        room_id: String,
        socket: Box<dyn SocketWrapper>,
        outbound_receiver: mpsc::UnboundedReceiver<String>,
        message_handler: Arc<dyn MessageHandler>,
    ) -> Self {
        Self {
            username,
            room_id,
            socket,
            outbound_receiver,
            message_handler,
        }
    }

    /// Run the connection - handles both sending and receiving until disconnect
    pub async fn run(mut self) -> Result<(), SocketError> {
        loop {
            tokio::select! {
                // Handle outbound messages (from our app to client)
                msg = self.outbound_receiver.recv() => {
                    match msg {
                        Some(message) => {
                            self.socket.send_message(message).await?
                        }
                        None => break, // Channel closed, disconnect
                    }
                }

                // Handle inbound messages (from client to our app)
                msg = self.socket.receive_message() => {
                    match msg {
                        Ok(Some(message)) => {
                            // Call the provided callback to handle the message
                            self.message_handler
                                .handle_message(&self.username, &self.room_id, message)
                                .await;
                        }
                        Ok(None) => break, // Client disconnected
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        // Clean disconnect
        let _ = self.socket.close().await;
        Ok(())
    }
}
