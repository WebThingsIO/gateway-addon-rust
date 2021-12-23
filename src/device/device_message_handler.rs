/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{client::Client, Device};
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DeviceRemoveActionRequest, DeviceRemoveActionResponseMessageData, DeviceRequestActionRequest,
    DeviceRequestActionResponseMessageData, DeviceSetPropertyCommand, Message as IPCMessage,
};

pub(crate) async fn handle_message(
    device: Arc<Mutex<Box<dyn Device>>>,
    client: Arc<Mutex<Client>>,
    message: IPCMessage,
) -> Result<(), String> {
    match message {
        IPCMessage::DeviceSetPropertyCommand(DeviceSetPropertyCommand { data, .. }) => {
            let property = device
                .lock()
                .await
                .device_handle_mut()
                .get_property(&data.property_name)
                .ok_or_else(|| {
                    format!(
                        "Could not update property {} of {}: not found",
                        data.property_name, data.device_id,
                    )
                })?;
            let mut property = property.lock().await;

            property.on_update(data.property_value.clone()).await?;

            property
                .property_handle_mut()
                .set_value(Some(data.property_value.clone()))
                .await
                .map_err(|err| {
                    format!(
                        "Could not update property {} of {}: {}",
                        data.property_name, data.device_id, err,
                    )
                })?;
            Ok(())
        }
        IPCMessage::DeviceRequestActionRequest(DeviceRequestActionRequest { data, .. }) => {
            let result = device
                .lock()
                .await
                .device_handle_mut()
                .request_action(
                    data.action_name.clone(),
                    data.action_id.clone(),
                    data.input.clone(),
                )
                .await;

            let reply = DeviceRequestActionResponseMessageData {
                plugin_id: data.plugin_id.clone(),
                adapter_id: data.adapter_id.clone(),
                device_id: data.device_id.clone(),
                action_name: data.action_name.clone(),
                action_id: data.action_id.clone(),
                success: result.is_ok(),
            }
            .into();

            client
                .lock()
                .await
                .send_message(&reply)
                .await
                .map_err(|err| format!("{:?}", err))?;

            result.map_err(|err| {
                format!(
                    "Failed to request action {} for device {}: {:?}",
                    data.action_name, data.device_id, err
                )
            })
        }
        IPCMessage::DeviceRemoveActionRequest(DeviceRemoveActionRequest { data, .. }) => {
            let result = device
                .lock()
                .await
                .device_handle_mut()
                .remove_action(data.action_name.clone(), data.action_id.clone())
                .await;

            let reply = DeviceRemoveActionResponseMessageData {
                plugin_id: data.plugin_id.clone(),
                adapter_id: data.adapter_id.clone(),
                device_id: data.device_id.clone(),
                action_name: data.action_name.clone(),
                action_id: data.action_id.clone(),
                message_id: data.message_id,
                success: result.is_ok(),
            }
            .into();

            client
                .lock()
                .await
                .send_message(&reply)
                .await
                .map_err(|err| format!("{:?}", err))?;

            result.map_err(|err| {
                format!(
                    "Failed to remove action {} ({}) for device {}: {:?}",
                    data.action_name, data.action_id, data.device_id, err
                )
            })
        }
        msg => Err(format!("Unexpected msg: {:?}", msg)),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        action::{tests::MockAction, Input, NoInput},
        adapter::tests::add_mock_device,
        device::tests::MockDevice,
        event::NoData,
        plugin::tests::{add_mock_adapter, plugin},
        property::{self, tests::MockProperty},
        EventHandle, Plugin, PropertyHandle,
    };
    use as_any::Downcast;
    use rstest::rstest;
    use serde_json::json;
    use webthings_gateway_ipc_types::{
        DeviceRemoveActionRequestMessageData, DeviceRequestActionRequestMessageData,
        DeviceSetPropertyCommandMessageData, Message,
    };

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const ACTION_ID: &str = "action_id";

    #[rstest]
    #[case(MockDevice::ACTION_NOINPUT, json!(null), NoInput)]
    #[case(MockDevice::ACTION_BOOL, json!(true), true)]
    #[case(MockDevice::ACTION_U8, json!(112_u8), 112_u8)]
    #[case(MockDevice::ACTION_I32, json!(21), 21)]
    #[case(MockDevice::ACTION_F32, json!(-2.7_f32), -2.7_f32)]
    #[case(MockDevice::ACTION_OPTI32, json!(11), Some(11))]
    #[case(MockDevice::ACTION_OPTI32, json!(null), Option::<i32>::None)]
    #[case(MockDevice::ACTION_STRING, json!("foo"), "foo".to_owned())]
    #[tokio::test]
    async fn test_request_action_perform<T: Input + PartialEq>(
        #[case] action_name: &'static str,
        #[case] action_input: serde_json::Value,
        #[case] expected_input: T,
        mut plugin: Plugin,
    ) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let mut device = device.lock().await;
            let action = device.device_handle_mut().get_action(action_name).unwrap();
            let mut action = action.lock().await;
            let action = action.as_any_mut().downcast_mut::<MockAction<T>>().unwrap();
            action
                .action_helper
                .expect_perform()
                .withf(move |action_handle| action_handle.input == expected_input)
                .times(1)
                .returning(|_| Ok(()));
        }

        let message: Message = DeviceRequestActionRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            action_name: action_name.to_owned(),
            action_id: ACTION_ID.to_owned(),
            input: action_input,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceRequestActionResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action_name == action_name
                        && msg.data.action_id == ACTION_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_action_cancel(mut plugin: Plugin) {
        let message_id = 42;
        let action_name = MockDevice::ACTION_I32.to_owned();
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let mut device = device.lock().await;
            let action = device
                .device_handle_mut()
                .get_action(action_name.to_owned())
                .unwrap();
            let mut action = action.lock().await;
            let action = action
                .as_any_mut()
                .downcast_mut::<MockAction<i32>>()
                .unwrap();
            action
                .action_helper
                .expect_cancel()
                .withf(move |action_id| action_id == ACTION_ID)
                .times(1)
                .returning(|_| Ok(()));
        }

        let message: Message = DeviceRemoveActionRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            action_name: action_name.to_owned(),
            action_id: ACTION_ID.to_owned(),
            message_id,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceRemoveActionResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action_name == action_name
                        && msg.data.action_id == ACTION_ID
                        && msg.data.message_id == message_id
                        && msg.data.success
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[case(MockDevice::PROPERTY_BOOL, json!(true), true)]
    #[case(MockDevice::PROPERTY_U8, json!(112_u8), 112_u8)]
    #[case(MockDevice::PROPERTY_I32, json!(21), 21)]
    #[case(MockDevice::PROPERTY_F32, json!(-2.7_f32), -2.7_f32)]
    #[case(MockDevice::PROPERTY_OPTI32, json!(11), Some(11))]
    #[case(MockDevice::PROPERTY_OPTI32, json!(null), Option::<i32>::None)]
    #[case(MockDevice::PROPERTY_STRING, json!("foo"), "foo".to_owned())]
    #[tokio::test]
    async fn test_request_property_update_value<T: property::Value + PartialEq>(
        #[case] property_name: &'static str,
        #[case] property_value: serde_json::Value,
        #[case] expected_value: T,
        mut plugin: Plugin,
    ) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let expected_value = expected_value.clone();
            let mut device = device.lock().await;
            let property = device
                .device_handle_mut()
                .get_property(property_name)
                .unwrap();
            let mut property = property.lock().await;
            let property = property.downcast_mut::<MockProperty<T>>().unwrap();
            property
                .property_helper
                .expect_on_update()
                .withf(move |value| value == &expected_value)
                .times(1)
                .returning(|_| Ok(()));
        }

        let serialized_value = property::Value::serialize(expected_value.clone()).unwrap();

        let message: Message = DeviceSetPropertyCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            property_name: property_name.to_owned(),
            property_value,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DevicePropertyChangedNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.property.name == Some(property_name.to_owned())
                        && msg.data.property.value == serialized_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_device_has_weak_adapter_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        assert!(device
            .lock()
            .await
            .device_handle_mut()
            .adapter
            .upgrade()
            .is_some())
    }

    #[rstest]
    #[tokio::test]
    async fn test_property_has_weak_device_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let property = device
            .lock()
            .await
            .device_handle_mut()
            .get_property(MockDevice::PROPERTY_I32)
            .unwrap();
        assert!(property
            .lock()
            .await
            .property_handle_mut()
            .downcast_ref::<PropertyHandle<i32>>()
            .unwrap()
            .device
            .upgrade()
            .is_some())
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_has_weak_device_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let event = device
            .lock()
            .await
            .device_handle_mut()
            .get_event(MockDevice::EVENT_NODATA)
            .unwrap();
        assert!(event
            .lock()
            .await
            .downcast_ref::<EventHandle<NoData>>()
            .unwrap()
            .device
            .upgrade()
            .is_some())
    }
}
