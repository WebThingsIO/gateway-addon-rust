/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{client::Client, error::WebthingsError};
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::ApiHandlerUnloadResponseMessageData;

/// A struct which represents an instance of a WebthingsIO API Handler.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct ApiHandlerHandle {
    pub(crate) client: Arc<Mutex<Client>>,
    pub plugin_id: String,
}

impl ApiHandlerHandle {
    pub(crate) fn new(client: Arc<Mutex<Client>>, plugin_id: String) -> Self {
        Self { client, plugin_id }
    }

    /// Unload this API Handler.
    pub async fn unload(&self) -> Result<(), WebthingsError> {
        let message = ApiHandlerUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            package_name: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{api_handler::ApiHandlerHandle, client::Client};
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    const PLUGIN_ID: &str = "plugin_id";

    #[fixture]
    fn api_handler() -> ApiHandlerHandle {
        let client = Arc::new(Mutex::new(Client::new()));
        ApiHandlerHandle::new(client, PLUGIN_ID.to_owned())
    }

    #[rstest]
    #[tokio::test]
    async fn test_unload(api_handler: ApiHandlerHandle) {
        api_handler
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::ApiHandlerUnloadResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID && msg.data.package_name == PLUGIN_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        api_handler.unload().await.unwrap();
    }
}
