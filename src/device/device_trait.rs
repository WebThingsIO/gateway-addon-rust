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
/// impl DeviceStructure for ExampleDevice {
///     // ...
///   # fn id(&self) -> String {
///   #     "example-device".to_owned()
///   # }
///   # fn description(&self) -> DeviceDescription {
///   #     DeviceDescription::default()
///   # }
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {}
/// ```
#[async_trait]
pub trait Device: BuiltDevice + Send + Sync + AsAny + 'static {}

impl Downcast for dyn Device {}

/// A trait used to wrap a [device handle][DeviceHandle].
///
/// When you use the [device][macro@crate::device] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::BuiltDevice};
/// # use async_trait::async_trait;
/// struct BuiltExampleDevice {
///     device_handle: DeviceHandle,
/// }
///
/// impl BuiltDevice for BuiltExampleDevice {
///     fn device_handle(&self) -> &DeviceHandle {
///         &self.device_handle
///     }
///     fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///         &mut self.device_handle
///     }
/// }
/// ```
pub trait BuiltDevice {
    /// Return a reference to the wrapped [device handle][DeviceHandle].
    fn device_handle(&self) -> &DeviceHandle;

    /// Return a mutable reference to the wrapped [device handle][DeviceHandle].
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::device::{tests::MockDevice, BuiltDevice, Device, DeviceHandle};

    pub struct BuiltMockDevice {
        data: MockDevice,
        device_handle: DeviceHandle,
    }

    impl BuiltMockDevice {
        pub fn new(data: MockDevice, device_handle: DeviceHandle) -> Self {
            Self {
                data,
                device_handle,
            }
        }
    }

    impl BuiltDevice for BuiltMockDevice {
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
