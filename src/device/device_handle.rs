/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    action::ActionBase,
    client::Client,
    error::WebthingsError,
    event::{EventBase, EventBuilderBase},
    property::{PropertyBase, PropertyBuilderBase},
    ActionHandle, Adapter, Device, DeviceDescription,
};

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{DeviceConnectedStateNotificationMessageData, Message};

/// A struct which represents an instance of a WoT device.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct DeviceHandle {
    pub(crate) client: Arc<Mutex<Client>>,
    pub(crate) weak: Weak<Mutex<Box<dyn Device>>>,
    /// Reference to the [adapter][crate::adapter::Adapter] which owns this device.
    pub adapter: Weak<Mutex<Box<dyn Adapter>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub description: DeviceDescription,
    pub connected: bool,
    properties: HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>>,
    actions: HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>>,
    events: HashMap<String, Arc<Mutex<Box<dyn EventBase>>>>,
}

impl DeviceHandle {
    pub(crate) fn new(
        client: Arc<Mutex<Client>>,
        adapter: Weak<Mutex<Box<dyn Adapter>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        description: DeviceDescription,
    ) -> Self {
        DeviceHandle {
            client,
            weak: Weak::new(),
            adapter,
            plugin_id,
            adapter_id,
            description,
            device_id,
            connected: true,
            properties: HashMap::new(),
            actions: HashMap::new(),
            events: HashMap::new(),
        }
    }

    pub(crate) async fn add_property(&mut self, property_builder: Box<dyn PropertyBuilderBase>) {
        let name = property_builder.name();

        let property = Arc::new(Mutex::new(property_builder.build(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.device_id.clone(),
        )));

        self.properties.insert(name, property.clone());
        property.lock().await.post_init();
    }

    /// Get a reference to all the [properties][crate::Property] which this device owns.
    pub fn properties(&self) -> &HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>> {
        &self.properties
    }

    /// Get a [property][crate::property::Property] which this device owns by ID.
    pub fn get_property(
        &self,
        name: impl Into<String>,
    ) -> Option<Arc<Mutex<Box<dyn PropertyBase>>>> {
        self.properties.get(&name.into()).cloned()
    }

    /// Helper method for setting the value of a [property][crate::Property] which this device owns by ID.
    ///
    /// Make sure that the type of the provided value is compatible with the respective property.
    pub async fn set_property_value(
        &self,
        name: impl Into<String>,
        value: Option<serde_json::Value>,
    ) -> Result<(), WebthingsError> {
        let name = name.into();
        if let Some(property) = self.properties.get(&name.clone()) {
            let mut property = property.lock().await;
            property.property_handle_mut().set_value(value).await?;
            Ok(())
        } else {
            Err(WebthingsError::UnknownProperty(name))
        }
    }

    pub(crate) async fn add_action(&mut self, action: Box<dyn ActionBase>) {
        let name = action.name();

        let action = Arc::new(Mutex::new(action));

        self.actions.insert(name, action.clone());
        action.lock().await.post_init();
    }

    /// Get a reference to all the [actions][crate::action::Action] which this device owns.
    pub fn actions(&self) -> &HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>> {
        &self.actions
    }

    /// Get an [action][crate::Action] which this device owns by ID.
    pub fn get_action(&self, name: impl Into<String>) -> Option<Arc<Mutex<Box<dyn ActionBase>>>> {
        self.actions.get(&name.into()).cloned()
    }

    pub(crate) async fn request_action(
        &self,
        action_name: String,
        action_id: String,
        input: serde_json::Value,
    ) -> Result<(), String> {
        let action = self.get_action(&action_name).ok_or_else(|| {
            format!(
                "Failed to request action {} of {}: not found",
                action_name, self.device_id,
            )
        })?;
        let mut action = action.lock().await;
        let action_handle = ActionHandle::new(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.device_id.clone(),
            action.name(),
            action_id,
            input.clone(),
            input,
        );
        action.check_and_perform(action_handle).await
    }

    pub(crate) async fn remove_action(
        &self,
        action_name: String,
        action_id: String,
    ) -> Result<(), String> {
        let action = self.get_action(&action_name).ok_or_else(|| {
            format!(
                "Failed to remove action {} ({}) of {}: not found",
                action_name, action_id, self.device_id,
            )
        })?;
        let mut action = action.lock().await;
        action.cancel(action_id).await
    }

    pub(crate) async fn add_event(&mut self, event_builder: Box<dyn EventBuilderBase>) {
        let name = event_builder.name();

        let event = Arc::new(Mutex::new(event_builder.build(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.device_id.clone(),
        )));

        self.events.insert(name, event.clone());

        event.lock().await.post_init();
    }

    /// Get a reference to all the [events][crate::event::Event] which this device owns.
    pub fn events(&self) -> &HashMap<String, Arc<Mutex<Box<dyn EventBase>>>> {
        &self.events
    }

    /// Get an [event][crate::Event] which this device owns by ID.
    pub fn get_event(&self, name: impl Into<String>) -> Option<Arc<Mutex<Box<dyn EventBase>>>> {
        self.events.get(&name.into()).cloned()
    }

    /// Helper method for raising an [event][crate::event::Event] which this device owns by ID.
    ///
    /// Make sure that the type of the provided data is compatible with the respective event.
    pub async fn raise_event(
        &self,
        name: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Result<(), WebthingsError> {
        let name = name.into();
        if let Some(event) = self.events.get(&name.clone()) {
            let event = event.lock().await;
            event.event_handle().raise(data).await?;
            Ok(())
        } else {
            Err(WebthingsError::UnknownEvent(name))
        }
    }

    /// Set the connected state of this device and notify the gateway.
    pub async fn set_connected(&mut self, connected: bool) -> Result<(), WebthingsError> {
        self.connected = connected;

        let message: Message = DeviceConnectedStateNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            connected,
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        action::{tests::MockAction, NoInput},
        client::Client,
        event::{tests::MockEvent, NoData},
        property::tests::MockProperty,
        DeviceDescription, DeviceHandle,
    };
    use rstest::{fixture, rstest};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const PROPERTY_NAME: &str = "property_name";
    const ACTION_NAME: &str = "action_name";
    const EVENT_NAME: &str = "event_name";

    #[fixture]
    fn device() -> DeviceHandle {
        let client = Arc::new(Mutex::new(Client::new()));
        let device_description = DeviceDescription::default();
        DeviceHandle::new(
            client,
            Weak::new(),
            PLUGIN_ID.to_owned(),
            ADAPTER_ID.to_owned(),
            DEVICE_ID.to_owned(),
            device_description,
        )
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_property(mut device: DeviceHandle) {
        device
            .add_property(Box::new(MockProperty::<i32>::new(PROPERTY_NAME.to_owned())))
            .await;
        assert!(device.get_property(PROPERTY_NAME).is_some())
    }

    #[rstest]
    fn test_get_unknown_property(device: DeviceHandle) {
        assert!(device.get_property(PROPERTY_NAME).is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_action(mut device: DeviceHandle) {
        device
            .add_action(Box::new(MockAction::<NoInput>::new(ACTION_NAME.to_owned())))
            .await;
        assert!(device.get_action(ACTION_NAME).is_some())
    }

    #[rstest]
    fn test_get_unknown_action(device: DeviceHandle) {
        assert!(device.get_action(ACTION_NAME).is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_event(mut device: DeviceHandle) {
        device
            .add_event(Box::new(MockEvent::<NoData>::new(EVENT_NAME.to_owned())))
            .await;
        assert!(device.get_event(EVENT_NAME).is_some())
    }

    #[rstest]
    fn test_get_unknown_event(device: DeviceHandle) {
        assert!(device.get_event(EVENT_NAME).is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_set_property_value(mut device: DeviceHandle) {
        let value = 42;
        device
            .add_property(Box::new(MockProperty::<i32>::new(PROPERTY_NAME.to_owned())))
            .await;

        device
            .client
            .lock()
            .await
            .expect_send_message()
            .times(1)
            .returning(|_| Ok(()));

        assert!(device
            .set_property_value(PROPERTY_NAME, Some(json!(value)))
            .await
            .is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn test_set_unknown_property_value(device: DeviceHandle) {
        let value = 42;
        assert!(device
            .set_property_value(PROPERTY_NAME, Some(json!(value)))
            .await
            .is_err());
    }

    #[rstest]
    #[tokio::test]
    async fn test_raise_event(mut device: DeviceHandle) {
        device
            .add_event(Box::new(MockEvent::<NoData>::new(EVENT_NAME.to_owned())))
            .await;

        device
            .client
            .lock()
            .await
            .expect_send_message()
            .times(1)
            .returning(|_| Ok(()));

        assert!(device.raise_event(EVENT_NAME, None).await.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn test_raise_unknown_event(device: DeviceHandle) {
        assert!(device.raise_event(EVENT_NAME, None).await.is_err());
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    #[tokio::test]
    async fn test_set_connected(mut device: DeviceHandle, #[case] connected: bool) {
        device
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceConnectedStateNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.connected == connected
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        assert!(device.set_connected(connected).await.is_ok());
        assert_eq!(device.connected, connected);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_post_init(mut device: DeviceHandle) {
        let mut mock_event = MockEvent::<NoData>::new(EVENT_NAME.to_owned());
        mock_event.expect_post_init = true;
        mock_event.expect_post_init().times(1).returning(|| ());
        device.add_event(Box::new(mock_event)).await;
    }

    #[rstest]
    #[tokio::test]
    async fn test_action_post_init(mut device: DeviceHandle) {
        let mut mock_action = MockAction::<NoInput>::new(ACTION_NAME.to_owned());
        mock_action.expect_post_init = true;
        mock_action.expect_post_init().times(1).returning(|| ());
        device.add_action(Box::new(mock_action)).await;
    }

    #[rstest]
    #[tokio::test]
    async fn test_property_post_init(mut device: DeviceHandle) {
        let mut mock_property = MockProperty::<i32>::new(PROPERTY_NAME.to_owned());
        mock_property.expect_post_init = true;
        mock_property.expect_post_init().times(1).returning(|| ());
        device.add_property(Box::new(mock_property)).await;
    }
}
