/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    property::{PropertyHandleBase, Value},
    PropertyHandle,
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;

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

#[cfg(test)]
pub(crate) mod tests {
    use crate::{property, Property, PropertyHandle};
    use async_trait::async_trait;
    use mockall::mock;

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
}
