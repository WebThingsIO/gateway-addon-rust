/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    property::{PropertyHandleBase, Value},
    PropertyHandle, PropertyStructure,
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;

/// A trait used to specify the behaviour of a WoT property.
///
/// Defines how to react on gateway requests. Built by a [crate::PropertyBuilder].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::PropertyHandleWrapper};
/// # use async_trait::async_trait;
/// #[property]
/// struct ExampleProperty {
///     foo: i32,
/// }
///
/// impl PropertyStructure for ExampleProperty {
///     // ...
///     # type Value = i32;
///     # fn name(&self) -> String {
///     #     "example-property".to_owned()
///     # }
///     # fn description(&self) -> PropertyDescription<Self::Value> {
///     #     PropertyDescription::default()
///     # }
/// }
///
/// #[async_trait]
/// impl Property for BuiltExampleProperty {
///     async fn on_update(&mut self, value: Self::Value) -> Result<(), String> {
///         log::debug!(
///             "Value with foo {:?} changed from {:?} to {:?}",
///             self.foo,
///             self.property_handle().description.value,
///             value,
///         );
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Property: PropertyHandleWrapper + Send + Sync + 'static {
    /// Called when the [value][Value] has been updated through the gateway.
    ///
    /// Should return `Ok(())` when the given value is accepted and an `Err` otherwise.
    async fn on_update(
        &mut self,
        _value: <Self as PropertyHandleWrapper>::Value,
    ) -> Result<(), String> {
        Ok(())
    }
}

/// An object safe variant of [Property] + [PropertyHandleWrapper].
///
/// Auto-implemented for all objects which implement the [Property] trait. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [Property] implementation.
///
/// This can (in contrast to the [Property] trait) be used to store objects for dynamic dispatch.
#[async_trait]
pub trait PropertyBase: Send + Sync + AsAny + 'static {
    /// Return a reference to the wrapped [property handle][PropertyHandle].
    fn property_handle(&self) -> &dyn PropertyHandleBase;

    /// Return a mutable reference to the wrapped [property handle][PropertyHandle].
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase;

    #[doc(hidden)]
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String>;
}

impl Downcast for dyn PropertyBase {}

#[async_trait]
impl<T: Property> PropertyBase for T {
    fn property_handle(&self) -> &dyn PropertyHandleBase {
        <T as PropertyHandleWrapper>::property_handle(self)
    }

    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase {
        <T as PropertyHandleWrapper>::property_handle_mut(self)
    }

    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String> {
        let value = <T as PropertyHandleWrapper>::Value::deserialize(Some(value))
            .map_err(|err| format!("Could not deserialize value: {:?}", err))?;
        <T as Property>::on_update(self, value).await
    }
}

/// A trait used to wrap a [property handle][PropertyHandle].
///
/// When you use the [property][macro@crate::property] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::PropertyHandleWrapper};
/// # use async_trait::async_trait;
/// struct BuiltExampleProperty {
///     property_handle: PropertyHandle<i32>,
/// }
///
/// impl PropertyHandleWrapper for BuiltExampleProperty {
///     type Value = i32;
///     fn property_handle(&self) -> &PropertyHandle<Self::Value> {
///         &self.property_handle
///     }
///     fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
///         &mut self.property_handle
///     }
/// }
/// ```
pub trait PropertyHandleWrapper {
    /// Type of [value][Value] of wrapped [property handle][PropertyHandle].
    type Value: Value;

    /// Return a reference to the wrapped [property handle][PropertyHandle].
    fn property_handle(&self) -> &PropertyHandle<Self::Value>;

    /// Return a mutable reference to the wrapped [property handle][PropertyHandle].
    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value>;
}

/// A trait used to build a [Property] around a data struct and a [property handle][PropertyHandle].
///
/// When you use the [property][macro@crate::property] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::{PropertyHandleWrapper, BuildProperty}};
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
/// impl PropertyHandleWrapper for BuiltExampleProperty {
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
/// impl BuildProperty for ExampleProperty {
///     type BuiltProperty = BuiltExampleProperty;
///     fn build(data: Self, property_handle: PropertyHandle<i32>) -> Self::BuiltProperty {
///         BuiltExampleProperty {
///             data,
///             property_handle,
///         }
///     }
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
/// ```
pub trait BuildProperty: PropertyStructure {
    /// Type of [Property] to build.
    type BuiltProperty: Property;

    /// Build the [property][Property] from a data struct and an [property handle][PropertyHandle].
    fn build(
        data: Self,
        property_handle: PropertyHandle<<Self as PropertyStructure>::Value>,
    ) -> Self::BuiltProperty;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        property::{self, tests::MockProperty, PropertyHandleWrapper},
        Property, PropertyHandle,
    };
    use async_trait::async_trait;

    pub struct BuiltMockProperty<T: property::Value> {
        data: MockProperty<T>,
        property_handle: PropertyHandle<T>,
    }

    impl<T: property::Value> BuiltMockProperty<T> {
        pub fn new(data: MockProperty<T>, property_handle: PropertyHandle<T>) -> Self {
            Self {
                data,
                property_handle,
            }
        }
    }

    impl<T: property::Value> PropertyHandleWrapper for BuiltMockProperty<T> {
        type Value = T;

        fn property_handle(&self) -> &PropertyHandle<T> {
            &self.property_handle
        }

        fn property_handle_mut(&mut self) -> &mut PropertyHandle<T> {
            &mut self.property_handle
        }
    }

    impl<T: property::Value> std::ops::Deref for BuiltMockProperty<T> {
        type Target = MockProperty<T>;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl<T: property::Value> std::ops::DerefMut for BuiltMockProperty<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    #[async_trait]
    impl<T: property::Value> Property for BuiltMockProperty<T> {
        async fn on_update(&mut self, value: Self::Value) -> Result<(), String> {
            self.property_helper.on_update(value)
        }
    }
}
