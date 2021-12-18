/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::error::WebthingsError;
use futures::{prelude::*, stream::SplitSink};
use mockall_double::double;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use webthings_gateway_ipc_types::Message as IPCMessage;

#[cfg(test)]
mockall::mock! {
    pub WebsocketClient {
        pub async fn send_message(&mut self, msg: &IPCMessage) -> Result<(), WebthingsError>;
    }
}

pub struct WebsocketClient {
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

impl WebsocketClient {
    pub fn new(sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> Self {
        Self { sink }
    }

    pub async fn send(&mut self, msg: String) -> Result<(), WebthingsError> {
        log::trace!("Sending message {}", msg);

        self.sink
            .send(Message::Text(msg))
            .await
            .map_err(WebthingsError::Send)
    }

    pub async fn send_message(&mut self, msg: &IPCMessage) -> Result<(), WebthingsError> {
        let json = serde_json::to_string(msg).map_err(WebthingsError::Serialization)?;

        self.send(json).await
    }
}

#[double]
pub use WebsocketClient as Client;
