/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use async_trait::async_trait;
use webthings_gateway_ipc_types::Message as IPCMessage;

#[cfg(not(test))]
use {
    futures::{prelude::*, stream::SplitSink},
    tokio::net::TcpStream,
    tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream},
};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ClientExt: Send + Sync + 'static {
    async fn send_message(&mut self, msg: &IPCMessage) -> Result<(), ApiError>;
}

#[cfg(not(test))]
pub struct WebsocketClient {
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

#[cfg(not(test))]
impl WebsocketClient {
    pub fn new(sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> Self {
        Self { sink }
    }

    pub async fn send(&mut self, msg: String) -> Result<(), ApiError> {
        log::trace!("Sending message {}", msg);

        self.sink
            .send(Message::Text(msg))
            .await
            .map_err(ApiError::Send)
    }
}

#[cfg(not(test))]
#[async_trait]
impl ClientExt for WebsocketClient {
    async fn send_message(&mut self, msg: &IPCMessage) -> Result<(), ApiError> {
        let json = serde_json::to_string(msg).map_err(ApiError::Serialization)?;

        self.send(json).await
    }
}

#[cfg(test)]
pub type Client = MockClientExt;
#[cfg(not(test))]
pub type Client = WebsocketClient;
