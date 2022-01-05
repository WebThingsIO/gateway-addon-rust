/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    client::Client,
    error::WebthingsError,
    property::{PropertyBase, Value},
    Device, Property, PropertyDescription, PropertyHandle,
};
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Property as FullPropertyDescription;

/// A trait used to specify the structure of a WoT property.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// pub struct ExampleProperty {
///     foo: i32,
/// }
///
/// impl PropertyStructure for ExampleProperty {
///     type Value = i32;
///
///     fn name(&self) -> String {
///         "example-property".to_owned()
///     }
///
///     fn description(&self) -> PropertyDescription<i32> {
///         PropertyDescription::default()
///     }
/// }
/// ```
pub trait PropertyStructure: Send + Sync + 'static {
    /// Type of [value][Value] which `Self::Property` accepts.
    type Value: Value;

    /// Name of the property.
    fn name(&self) -> String;

    /// [WoT description][PropertyDescription] of the property.
    fn description(&self) -> PropertyDescription<Self::Value>;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, WebthingsError> {
        self.description().into_full_description(self.name())
    }
}

/// A trait used to build a [Property] around a data struct and a [property handle][PropertyHandle].
///
/// When you use the [property][macro@crate::property] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::{BuiltProperty, PropertyBuilder}};
/// # use async_trait::async_trait;
/// struct ExampleProperty {
///     foo: i32,
/// }
///
/// struct BuiltExampleProperty {
///     data: ExampleProperty,
///     property_handle: PropertyHandle<i32>,
/// }
///
/// impl BuiltProperty for BuiltExampleProperty {
///     // ...
///   # type Value = i32;
///   # fn property_handle(&self) -> &PropertyHandle<i32> {
///   #     &self.property_handle
///   # }
///   # fn property_handle_mut(&mut self) -> &mut PropertyHandle<i32> {
///   #     &mut self.property_handle
///   # }
/// }
///
/// impl PropertyStructure for ExampleProperty {
///     /// ...
/// #   type Value = i32;
/// #   fn name(&self) -> String {
/// #       "example-property".to_owned()
/// #   }
/// #   fn description(&self) -> PropertyDescription<Self::Value> {
/// #       PropertyDescription::default()
/// #   }
/// }
///
/// #[async_trait]
/// impl Property for BuiltExampleProperty {}
///
/// impl PropertyBuilder for ExampleProperty {
///     type BuiltProperty = BuiltExampleProperty;
///     fn build(data: Self, property_handle: PropertyHandle<i32>) -> Self::BuiltProperty {
///         BuiltExampleProperty {
///             data,
///             property_handle,
///         }
///     }
/// }
/// ```
pub trait PropertyBuilder: PropertyStructure {
    /// Type of [Property] to build.
    type BuiltProperty: Property;

    /// Build the [property][Property] from a data struct and an [property handle][PropertyHandle].
    fn build(
        data: Self,
        property_handle: PropertyHandle<<Self as PropertyStructure>::Value>,
    ) -> Self::BuiltProperty;
}

/// An object safe variant of [PropertyBuilder] + [PropertyStructure].
///
/// Auto-implemented for all objects which implement the [PropertyBuilder] trait. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [PropertyBuilder] / [PropertyStructure] implementation.
///
/// This can (in contrast to to the [PropertyBuilder] trait) be used to store objects for dynamic dispatch.
pub trait PropertyBuilderBase: Send + Sync + 'static {
    /// Name of the property.
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, WebthingsError>;

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
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
        <T as PropertyStructure>::name(self)
    }

    fn full_description(&self) -> Result<FullPropertyDescription, WebthingsError> {
        <T as PropertyStructure>::full_description(self)
    }

    fn build(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn PropertyBase> {
        let property_handle = PropertyHandle::<<Self as PropertyStructure>::Value>::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            self.name(),
            self.description(),
        );
        Box::new(<T as PropertyBuilder>::build(*self, property_handle))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::ops::{Deref, DerefMut};

    use crate::{
        property::{self, tests::BuiltMockProperty, PropertyBuilder},
        PropertyDescription, PropertyHandle, PropertyStructure,
    };
    use mockall::mock;

    mock! {
        pub PropertyHelper<T> {
            pub fn on_update(&self, value: T) -> Result<(), String>;
        }
    }

    pub struct MockProperty<T: property::Value> {
        property_name: String,
        pub property_helper: MockPropertyHelper<T>,
    }

    impl<T: property::Value> MockProperty<T> {
        pub fn new(property_name: String) -> Self {
            Self {
                property_name,
                property_helper: MockPropertyHelper::new(),
            }
        }
    }

    impl<T: property::Value> Deref for MockProperty<T> {
        type Target = MockPropertyHelper<T>;

        fn deref(&self) -> &Self::Target {
            &self.property_helper
        }
    }

    impl<T: property::Value> DerefMut for MockProperty<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.property_helper
        }
    }

    impl<T: property::Value> PropertyStructure for MockProperty<T> {
        type Value = T;

        fn name(&self) -> String {
            self.property_name.to_owned()
        }

        fn description(&self) -> PropertyDescription<Self::Value> {
            PropertyDescription::default()
        }
    }

    impl<T: property::Value> PropertyBuilder for MockProperty<T> {
        type BuiltProperty = BuiltMockProperty<T>;
        fn build(data: Self, property_handle: PropertyHandle<T>) -> Self::BuiltProperty {
            BuiltMockProperty::new(data, property_handle)
        }
    }
}
