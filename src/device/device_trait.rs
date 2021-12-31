/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::DeviceHandle;
use as_any::{AsAny, Downcast};
use async_trait::async_trait;

/// A trait used to specify the behaviour of a WoT device.
///
/// Defines how to react on gateway requests. Built by an [adapter][crate::Adapter].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// # use async_trait::async_trait;
/// #[device]
/// struct ExampleDevice {
///     foo: i32,
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {}
/// ```
#[async_trait]
pub trait Device: DeviceHandleWrapper + Send + Sync + AsAny + 'static {}

impl Downcast for dyn Device {}

/// A trait used to wrap a [device handle][DeviceHandle].
///
/// When you use the [device][macro@crate::device] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::DeviceHandleWrapper};
/// # use async_trait::async_trait;
/// struct BuiltExampleDevice {
///     device_handle: DeviceHandle,
/// }
///
/// impl DeviceHandleWrapper for BuiltExampleDevice {
///     fn device_handle(&self) -> &DeviceHandle {
///         &self.device_handle
///     }
///     fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///         &mut self.device_handle
///     }
/// }
/// ```
pub trait DeviceHandleWrapper {
    /// Return a reference to the wrapped [device handle][DeviceHandle].
    fn device_handle(&self) -> &DeviceHandle;

    /// Return a mutable reference to the wrapped [device handle][DeviceHandle].
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
}

/// A trait used to build a [Device] around a data struct and a [device handle][DeviceHandle].
///
/// When you use the [device][macro@crate::device] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::{DeviceHandleWrapper, BuildDevice}};
/// # use async_trait::async_trait;
/// struct ExampleDevice {
///     foo: i32,
/// }
///
/// struct BuiltExampleDevice {
///     data: ExampleDevice,
///     device_handle: DeviceHandle,
/// }
///
/// impl DeviceHandleWrapper for BuiltExampleDevice {
///     // ...
///   # fn device_handle(&self) -> &DeviceHandle {
///   #     &self.device_handle
///   # }
///   # fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///   #     &mut self.device_handle
///   # }
/// }
///
/// impl BuildDevice for ExampleDevice {
///     type BuiltDevice = BuiltExampleDevice;
///     fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
///         BuiltExampleDevice {
///             data,
///             device_handle,
///         }
///     }
/// }
///
/// impl DeviceStructure for ExampleDevice {
///     /// ...
/// #   fn id(&self) -> String {
/// #       "example-device".to_owned()
/// #   }
/// #   fn description(&self) -> DeviceDescription {
/// #       DeviceDescription::default()
/// #   }
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {}
/// ```
pub trait BuildDevice {
    /// Type of [Device] to build.
    type BuiltDevice: Device;

    /// Build the [device][Device] from a data struct and an [device handle][DeviceHandle].
    fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::device::{
        tests::MockDevice, BuildDevice, Device, DeviceHandle, DeviceHandleWrapper,
    };

    pub struct BuiltMockDevice {
        data: MockDevice,
        device_handle: DeviceHandle,
    }

    impl BuildDevice for MockDevice {
        type BuiltDevice = BuiltMockDevice;
        fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
            BuiltMockDevice {
                data,
                device_handle,
            }
        }
    }

    impl DeviceHandleWrapper for BuiltMockDevice {
        fn device_handle(&self) -> &DeviceHandle {
            &self.device_handle
        }

        fn device_handle_mut(&mut self) -> &mut DeviceHandle {
            &mut self.device_handle
        }
    }

    impl std::ops::Deref for BuiltMockDevice {
        type Target = MockDevice;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl std::ops::DerefMut for BuiltMockDevice {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    impl Device for BuiltMockDevice {}
}
