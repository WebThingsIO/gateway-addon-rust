/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use mockall_double::double;

mod double {
    #[cfg(not(test))]
    pub mod plugin {
        use crate::{
            api_handler::{ApiHandlerBuilder, ApiHandlerHandle, NoopApiHandler},
            client::Client,
            error::WebthingsError,
            Plugin,
        };
        use futures::stream::{SplitStream, StreamExt};
        use std::{collections::HashMap, str::FromStr, sync::Arc};
        use tokio::{net::TcpStream, sync::Mutex};
        use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
        use url::Url;
        use webthings_gateway_ipc_types::{
            Message as IPCMessage, PluginRegisterRequestMessageData,
            PluginRegisterResponseMessageData,
        };

        pub(crate) type PluginStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
        const GATEWAY_URL: &str = "ws://localhost:9500";

        /// Connect to a WebthingsIO gateway and create a new [plugin][Plugin].
        pub async fn connect(plugin_id: impl Into<String>) -> Result<Plugin, WebthingsError> {
            let plugin_id = plugin_id.into();
            let url = Url::parse(GATEWAY_URL).expect("Could not parse url");

            let (socket, _) = connect_async(url).await.map_err(WebthingsError::Connect)?;

            let (sink, mut stream) = socket.split();
            let mut client = Client::new(sink);

            let message: IPCMessage = PluginRegisterRequestMessageData {
                plugin_id: plugin_id.clone(),
            }
            .into();

            client.send_message(&message).await?;

            let PluginRegisterResponseMessageData {
                gateway_version: _,
                plugin_id: _,
                preferences,
                user_profile,
            } = loop {
                match read(&mut stream).await {
                    None => {}
                    Some(result) => match result {
                        Ok(IPCMessage::PluginRegisterResponse(msg)) => {
                            break msg.data;
                        }
                        Ok(msg) => {
                            log::warn!("Received unexpected message {:?}", msg);
                        }
                        Err(err) => log::error!("Could not read message: {}", err),
                    },
                }
            };

            let client = Arc::new(Mutex::new(client));
            let api_handler = Arc::new(Mutex::new(NoopApiHandler::build(
                NoopApiHandler,
                ApiHandlerHandle::new(client.clone(), plugin_id.clone()),
            )));

            Ok(Plugin {
                plugin_id,
                preferences,
                user_profile,
                client,
                stream,
                adapters: HashMap::new(),
                api_handler,
            })
        }

        pub(crate) async fn read(stream: &mut PluginStream) -> Option<Result<IPCMessage, String>> {
            stream.next().await.map(|result| match result {
                Ok(msg) => {
                    let json = msg
                        .to_text()
                        .map_err(|err| format!("Could not get text message: {:?}", err))?;

                    log::trace!("Received message {}", json);

                    IPCMessage::from_str(json)
                        .map_err(|err| format!("Could not parse message: {:?}", err))
                }
                Err(err) => Err(err.to_string()),
            })
        }
    }

    #[cfg(test)]
    pub mod mock_plugin {
        use crate::{
            api_handler::{ApiHandlerBuilder, ApiHandlerHandle, NoopApiHandler},
            client::Client,
            Plugin,
        };
        use std::{collections::HashMap, sync::Arc};
        use tokio::sync::Mutex;
        use webthings_gateway_ipc_types::{Message as IPCMessage, Preferences, Units, UserProfile};

        pub(crate) type PluginStream = ();

        pub fn connect(plugin_id: impl Into<String>) -> Plugin {
            let plugin_id = plugin_id.into();
            let preferences = Preferences {
                language: "en-US".to_owned(),
                units: Units {
                    temperature: "degree celsius".to_owned(),
                },
            };
            let user_profile = UserProfile {
                addons_dir: "".to_owned(),
                base_dir: "".to_owned(),
                config_dir: "".to_owned(),
                data_dir: "".to_owned(),
                gateway_dir: "".to_owned(),
                log_dir: "".to_owned(),
                media_dir: "".to_owned(),
            };
            let client = Arc::new(Mutex::new(Client::new()));
            let api_handler = Arc::new(Mutex::new(NoopApiHandler::build(
                NoopApiHandler,
                ApiHandlerHandle::new(client.clone(), plugin_id.clone()),
            )));
            Plugin {
                plugin_id,
                preferences,
                user_profile,
                client,
                stream: (),
                adapters: HashMap::new(),
                api_handler,
            }
        }

        pub(crate) async fn read(_stream: &mut PluginStream) -> Option<Result<IPCMessage, String>> {
            None
        }
    }
}

#[double]
use double::plugin;

pub use plugin::*;
