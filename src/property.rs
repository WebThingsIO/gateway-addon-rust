/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT properties.

pub use crate::property_description::*;
use crate::{
    api_error::ApiError,
    client::{Client, ClientExt},
    device::Device,
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as FullPropertyDescription,
};

/// A trait used to specify the behaviour of a WoT property.
///
/// Wraps a [property handle][PropertyHandle] and defines how to react on gateway requests. Built by a [PropertyBuilder].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*};
/// # use async_trait::async_trait;
/// struct ExampleProperty(PropertyHandle<i32>);
///
/// #[async_trait]
/// impl Property for ExampleProperty {
///     type Value = i32;
///
///     fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
///         &mut self.0
///     }
///
///     async fn on_update(&mut self, value: Self::Value) -> Result<(), String> {
///         log::debug!(
///             "Value changed from {:?} to {:?}",
///             self.0.description.value,
///             value,
///         );
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Property: Send + Sync + 'static {
    /// Type of [value][Value] this property accepts.
    type Value: Value;

    /// Return the wrapped [property handle][PropertyHandle].
    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value>;

    /// Called when the [value][Value] has been updated through the gateway.
    ///
    /// Should return `Ok(())` when the given value is accepted and an `Err` otherwise.
    async fn on_update(&mut self, _value: Self::Value) -> Result<(), String> {
        Ok(())
    }
}

/// An object safe variant of [Property].
///
/// Auto-implemented for all objects which implement the [Property] trait. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [Property] implementation.
///
/// This can (in contrast to the [Property] trait) be used to store objects for dynamic dispatch.
#[async_trait]
pub trait PropertyBase: Send + Sync + AsAny + 'static {
    /// Return the wrapped [property handle][PropertyHandle].
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase;

    #[doc(hidden)]
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String>;
}

impl Downcast for dyn PropertyBase {}

#[async_trait]
impl<T: Property> PropertyBase for T {
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase {
        <T as Property>::property_handle_mut(self)
    }

    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String> {
        let value = T::Value::deserialize(Some(value))
            .map_err(|err| format!("Could not deserialize value: {:?}", err))?;
        <T as Property>::on_update(self, value).await
    }
}

/// A struct which represents an instance of a WoT property.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct PropertyHandle<T: Value> {
    client: Arc<Mutex<Client>>,
    /// Reference to the [device][crate::device::Device] which owns this property.
    pub device: Weak<Mutex<Box<dyn Device>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: PropertyDescription<T>,
    _value: PhantomData<T>,
}

impl<T: Value> PropertyHandle<T> {
    pub(crate) fn new(
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: PropertyDescription<T>,
    ) -> Self {
        PropertyHandle {
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
            _value: PhantomData,
        }
    }

    /// Sets the [value][Value] and notifies the gateway.
    pub async fn set_value(&mut self, value: T) -> Result<(), ApiError> {
        self.description.value = value;

        let message: Message = DevicePropertyChangedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            property: self
                .description
                .clone()
                .into_full_description(self.name.clone())?,
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

/// A non-generic variant of [PropertyHandle].
///
/// Auto-implemented for every [PropertyHandle]. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [PropertyHandle] implementation.
#[async_trait]
pub trait PropertyHandleBase: Send + Sync + AsAny + 'static {
    /// Sets the [value][Value] and notifies the gateway.
    ///
    /// Make sure that the type of the provided value is compatible.
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), ApiError>;
}

impl Downcast for dyn PropertyHandleBase {}

#[async_trait]
impl<T: Value> PropertyHandleBase for PropertyHandle<T> {
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), ApiError> {
        let value = <T as Value>::deserialize(value)?;
        PropertyHandle::set_value(self, value).await
    }
}

/// A trait used to specify the structure of a WoT property.
///
/// Builds a [Property] instance.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, example::ExampleProperty};
/// // ...
/// struct ExamplePropertyBuilder();
///
/// impl PropertyBuilder for ExamplePropertyBuilder {
///     type Property = ExampleProperty;
///     type Value = i32;
///
///     fn name(&self) -> String {
///         "example-property".to_owned()
///     }
///
///     fn description(&self) -> PropertyDescription<i32> {
///         PropertyDescription::default()
///     }
///
///     fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property {
///         ExampleProperty::new(property_handle)
///     }
/// }
/// ```
pub trait PropertyBuilder: Send + Sync + 'static {
    /// Type of [property][Property] this builds.
    type Property: Property<Value = Self::Value>;

    /// Type of [value][Value] which `Self::Property` accepts.
    type Value: Value;

    /// Name of the property.
    fn name(&self) -> String;

    /// [WoT description][PropertyDescription] of the property.
    fn description(&self) -> PropertyDescription<Self::Value>;

    /// Build a new instance of this property using the given [property handle][PropertyHandle].
    fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, ApiError> {
        self.description().into_full_description(self.name())
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    fn build_(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Self::Property {
        let property_handle = PropertyHandle::<Self::Value>::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            self.name(),
            self.description(),
        );
        self.build(property_handle)
    }
}

/// An object safe variant of [PropertyBuilder].
///
/// Auto-implemented for all objects which implement the [PropertyBuilder] trait.  **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [PropertyBuilder] implementation.
///
/// This can (in contrast to the [PropertyBuilder] trait) be used to store objects for dynamic dispatch.
pub trait PropertyBuilderBase: Send + Sync + 'static {
    /// Name of the property.
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, ApiError>;

    #[doc(hidden)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn PropertyBase>;
}

impl<T: PropertyBuilder> PropertyBuilderBase for T {
    fn name(&self) -> String {
        <T as PropertyBuilder>::name(self)
    }

    fn full_description(&self) -> Result<FullPropertyDescription, ApiError> {
        <T as PropertyBuilder>::full_description(self)
    }

    #[doc(hidden)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn PropertyBase> {
        Box::new(<T as PropertyBuilder>::build_(
            self, client, device, plugin_id, adapter_id, device_id,
        ))
    }
}

/// Convenience type for a collection of [PropertyBuilderBase].
pub type Properties = Vec<Box<dyn PropertyBuilderBase>>;

/// Convenience macro for building a [Properties].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, example::ExamplePropertyBuilder};
/// properties![ExamplePropertyBuilder::new()]
/// # ;
/// ```
#[macro_export]
macro_rules! properties [
    ($($e:expr),*) => ({
        let _temp: Properties = vec![$(Box::new($e)),*];
        _temp
    })
];

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        client::Client,
        property::{self, Property, PropertyBuilder, PropertyHandle},
        property_description::PropertyDescription,
    };
    use async_trait::async_trait;
    use mockall::mock;
    use serde_json::json;
    use std::{
        marker::PhantomData,
        sync::{Arc, Weak},
    };
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    pub struct MockPropertyBuilder<T: property::Value> {
        property_name: String,
        _value: PhantomData<T>,
    }

    impl<T: property::Value> MockPropertyBuilder<T> {
        pub fn new(property_name: String) -> Self {
            Self {
                property_name,
                _value: PhantomData,
            }
        }
    }

    impl<T: property::Value> PropertyBuilder for MockPropertyBuilder<T> {
        type Property = MockProperty<T>;
        type Value = T;

        fn name(&self) -> String {
            self.property_name.to_owned()
        }

        fn description(&self) -> PropertyDescription<Self::Value> {
            PropertyDescription::default()
        }

        fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property {
            MockProperty::new(property_handle)
        }
    }

    mock! {
        pub PropertyHelper<T> {
            pub fn on_update(&self, value: T) -> Result<(), String>;
        }
    }

    pub struct MockProperty<T: property::Value> {
        property_handle: PropertyHandle<T>,
        pub property_helper: MockPropertyHelper<T>,
    }

    impl<T: property::Value> MockProperty<T> {
        pub fn new(property_handle: PropertyHandle<T>) -> Self {
            MockProperty {
                property_handle,
                property_helper: MockPropertyHelper::new(),
            }
        }
    }

    #[async_trait]
    impl<T: property::Value> Property for MockProperty<T> {
        type Value = T;
        fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
            &mut self.property_handle
        }
        async fn on_update(&mut self, value: Self::Value) -> Result<(), String> {
            self.property_helper.on_update(value)
        }
    }

    #[tokio::test]
    async fn test_set_value() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(Client::new()));
        let value = 42;

        let property_description = PropertyDescription::<i32>::default();

        let mut property = PropertyHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            property_name.clone(),
            property_description,
        );

        let expected_value = Some(json!(value.clone()));

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DevicePropertyChangedNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.property.name == Some(property_name.clone())
                        && msg.data.property.value == expected_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        property.set_value(value).await.unwrap();
    }
}
