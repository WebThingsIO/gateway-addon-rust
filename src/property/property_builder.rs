/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    api_error::ApiError,
    client::Client,
    property::{PropertyBase, Value},
    Device, Property, PropertyHandle,
};

use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Property as FullPropertyDescription;

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
    fn description(&self) -> crate::PropertyDescription<Self::Value>;

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

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        property::{self, tests::MockProperty},
        PropertyBuilder, PropertyDescription, PropertyHandle,
    };

    use std::marker::PhantomData;

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
}
