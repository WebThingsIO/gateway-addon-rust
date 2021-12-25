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
/// Defines how to react on gateway requests. Built by a [device builder][crate::DeviceBuilder].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// # use async_trait::async_trait;
/// #[device]
/// struct ExampleDevice;
///
/// #[async_trait]
/// impl Device for ExampleDevice {}
/// ```
#[async_trait]
pub trait Device: DeviceHandleWrapper + Send + Sync + AsAny + 'static {}

impl Downcast for dyn Device {}

/// A trait used to wrap a [device handle][DeviceHandle].
pub trait DeviceHandleWrapper {
    /// Return a reference to the wrapped [device handle][DeviceHandle].
    fn device_handle(&self) -> &DeviceHandle;

    /// Return a mutable reference to the wrapped [device handle][DeviceHandle].
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::device::{Device, DeviceHandle, DeviceHandleWrapper};

    pub struct MockDevice {
        device_handle: DeviceHandle,
    }

    impl MockDevice {
        pub const PROPERTY_BOOL: &'static str = "property_bool";
        pub const PROPERTY_U8: &'static str = "property_u8";
        pub const PROPERTY_I32: &'static str = "property_i32";
        pub const PROPERTY_F32: &'static str = "property_f32";
        pub const PROPERTY_OPTI32: &'static str = "property_opti32";
        pub const PROPERTY_STRING: &'static str = "property_string";
        pub const ACTION_NOINPUT: &'static str = "action_noinput";
        pub const ACTION_BOOL: &'static str = "action_bool";
        pub const ACTION_U8: &'static str = "action_u8";
        pub const ACTION_I32: &'static str = "action_i32";
        pub const ACTION_F32: &'static str = "action_f32";
        pub const ACTION_OPTI32: &'static str = "action_opti32";
        pub const ACTION_STRING: &'static str = "action_string";
        pub const EVENT_NODATA: &'static str = "event_nodata";

        pub fn new(device_handle: DeviceHandle) -> Self {
            MockDevice { device_handle }
        }
    }

    impl DeviceHandleWrapper for MockDevice {
        fn device_handle(&self) -> &DeviceHandle {
            &self.device_handle
        }

        fn device_handle_mut(&mut self) -> &mut DeviceHandle {
            &mut self.device_handle
        }
    }

    impl Device for MockDevice {}
}
