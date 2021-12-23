/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    client::Client,
    device::{self, device_message_handler},
    error::WebthingsError,
    Adapter, Device, DeviceBuilder,
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterRemoveDeviceRequest, AdapterRemoveDeviceResponseMessageData, AdapterStartPairingCommand,
    AdapterUnloadRequest, AdapterUnloadResponseMessageData, DeviceAddedNotificationMessageData,
    DeviceRemoveActionRequest, DeviceRemoveActionRequestMessageData, DeviceRequestActionRequest,
    DeviceRequestActionRequestMessageData, DeviceSavedNotification, DeviceSetPropertyCommand,
    DeviceSetPropertyCommandMessageData, DeviceWithoutId, Message, Message as IPCMessage,
};

pub(crate) async fn handle_message(
    adapter: Arc<Mutex<Box<dyn Adapter>>>,
    client: Arc<Mutex<Client>>,
    message: IPCMessage,
) -> Result<(), String> {
    match &message {
        IPCMessage::AdapterUnloadRequest(AdapterUnloadRequest { data, .. }) => {
            log::info!("Received request to unload adapter '{}'", data.adapter_id);

            let mut adapter = adapter.lock().await;

            adapter
                .on_unload()
                .await
                .map_err(|err| format!("Could not unload adapter: {}", err))?;

            adapter
                .adapter_handle_mut()
                .unload()
                .await
                .map_err(|err| format!("Could not send unload response: {}", err))?;

            Ok(())
        }
        IPCMessage::DeviceSavedNotification(DeviceSavedNotification { data, .. }) => {
            adapter
                .lock()
                .await
                .on_device_saved(data.device_id.clone(), data.device.clone())
                .await
                .map_err(|err| format!("Error during adapter.on_device_saved: {}", err))?;
            Ok(())
        }
        IPCMessage::AdapterStartPairingCommand(AdapterStartPairingCommand { data, .. }) => {
            adapter
                .lock()
                .await
                .on_start_pairing(Duration::from_secs(data.timeout as u64))
                .await
                .map_err(|err| format!("Error during adapter.on_start_pairing: {}", err))?;
            Ok(())
        }
        IPCMessage::AdapterCancelPairingCommand(_) => {
            adapter
                .lock()
                .await
                .on_cancel_pairing()
                .await
                .map_err(|err| format!("Error during adapter.on_cancel_pairing: {}", err))?;
            Ok(())
        }
        IPCMessage::AdapterRemoveDeviceRequest(AdapterRemoveDeviceRequest { data, .. }) => {
            let mut adapter = adapter.lock().await;

            adapter
                .on_remove_device(data.device_id.clone())
                .await
                .map_err(|err| format!("Could not execute remove device callback: {}", err))?;

            adapter
                .adapter_handle_mut()
                .remove_device(&data.device_id)
                .await
                .map_err(|err| format!("Could not remove device from adapter handle: {}", err))?;

            Ok(())
        }
        IPCMessage::DeviceSetPropertyCommand(DeviceSetPropertyCommand {
            data: DeviceSetPropertyCommandMessageData { device_id, .. },
            ..
        })
        | IPCMessage::DeviceRequestActionRequest(DeviceRequestActionRequest {
            data: DeviceRequestActionRequestMessageData { device_id, .. },
            ..
        })
        | IPCMessage::DeviceRemoveActionRequest(DeviceRemoveActionRequest {
            data: DeviceRemoveActionRequestMessageData { device_id, .. },
            ..
        }) => {
            let device = adapter
                .lock()
                .await
                .adapter_handle_mut()
                .get_device(device_id);
            let device = device.ok_or_else(|| format!("Unknown device: {}", device_id))?;
            device_message_handler::handle_message(device, client.clone(), message).await
        }
        msg => Err(format!("Unexpected msg: {:?}", msg)),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        adapter::tests::{add_mock_device, MockAdapter},
        client::Client,
        device::tests::MockDeviceBuilder,
        plugin::tests::{add_mock_adapter, plugin},
        Adapter, AdapterHandle, Device, DeviceBuilder, Plugin,
    };
    use as_any::Downcast;
    use async_trait::async_trait;
    use mockall::mock;
    use rstest::{fixture, rstest};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{
        AdapterCancelPairingCommandMessageData, AdapterRemoveDeviceRequestMessageData,
        AdapterStartPairingCommandMessageData, AdapterUnloadRequestMessageData,
        DeviceSavedNotificationMessageData, DeviceWithoutId, Message,
    };

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";

    #[rstest]
    #[tokio::test]
    async fn test_request_remove_device(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let message: Message = AdapterRemoveDeviceRequestMessageData {
            device_id: DEVICE_ID.to_owned(),
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterRemoveDeviceResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_remove_device()
                .withf(move |device_id| device_id == DEVICE_ID)
                .times(1)
                .returning(|_| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();

        assert!(adapter
            .lock()
            .await
            .adapter_handle_mut()
            .get_device(DEVICE_ID)
            .is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_unload(mut plugin: Plugin) {
        add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterUnloadRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        let adapter = plugin.borrow_adapter(ADAPTER_ID).unwrap();
        adapter
            .lock()
            .await
            .downcast_mut::<MockAdapter>()
            .unwrap()
            .adapter_helper
            .expect_on_unload()
            .times(1)
            .returning(|| Ok(()));

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterUnloadResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID && msg.data.adapter_id == ADAPTER_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_start_pairing(mut plugin: Plugin) {
        let timeout = 5000;
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterStartPairingCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            timeout,
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_start_pairing()
                .withf(move |t| t.as_secs() == timeout as u64)
                .times(1)
                .returning(|_| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_cancel_pairing(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterCancelPairingCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_cancel_pairing()
                .times(1)
                .returning(|| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_notification_device_saved(mut plugin: Plugin) {
        let device_description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = DeviceSavedNotificationMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            device: device_description.clone(),
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_device_saved()
                .withf(move |id, description| id == DEVICE_ID && description == &device_description)
                .times(1)
                .returning(|_, _| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }
}
