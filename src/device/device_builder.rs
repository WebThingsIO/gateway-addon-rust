/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    actions, error::WebthingsError, events, properties, Actions, Device, DeviceDescription,
    DeviceHandle, Events, Properties,
};
use std::collections::BTreeMap;
use webthings_gateway_ipc_types::Device as FullDeviceDescription;

/// A trait used to specify the structure of a WoT device.
///
/// # Examples
/// ```
/// # #[macro_use]
/// # use gateway_addon_rust::{prelude::*, example::{ExampleProperty, ExampleEvent, ExampleAction}};
/// pub struct ExampleDevice { foo: i32 }
///
/// impl DeviceStructure for ExampleDevice {
///     fn id(&self) -> String {
///         "example-device".to_owned()
///     }
///
///     fn description(&self) -> DeviceDescription {
///         DeviceDescription::default()
///     }
///
///     fn properties(&self) -> Properties {
///         properties![ExampleProperty::new()]
///     }
///
///     fn actions(&self) -> Actions {
///         actions![ExampleAction::new()]
///     }
///
///     fn events(&self) -> Events {
///         events![ExampleEvent::new()]
///     }
/// }
/// ```
pub trait DeviceStructure: Send + Sync + 'static {
    /// ID of the device.
    fn id(&self) -> String;

    /// [WoT description][DeviceDescription] of the device.
    fn description(&self) -> DeviceDescription;

    /// A list of [properties][crate::property::PropertyBuilder] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [PropertyBuilderBase][crate::property::PropertyBuilderBase].
    /// You can use the convenienve macro [properties!][crate::properties] to create this list [PropertyBuilder][crate::property::PropertyBuilder]s.
    fn properties(&self) -> Properties {
        properties![]
    }

    /// A list of [actions][crate::Action] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [ActionBase][crate::action::ActionBase].
    /// You can use the convenienve macro [actions!][crate::actions] to create this list from [Action][crate::Action]s.
    fn actions(&self) -> Actions {
        actions![]
    }

    /// A list of [events][crate::Event] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [EventBase][crate::event::EventBase].
    /// You can use the convenienve macro [events!][crate::events] to create this list from [Event][crate::Event]s.
    fn events(&self) -> Events {
        events![]
    }

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullDeviceDescription, WebthingsError> {
        let mut property_descriptions = BTreeMap::new();
        for property_builder in self.properties() {
            property_descriptions.insert(
                property_builder.name(),
                property_builder.full_description()?,
            );
        }

        let mut action_descriptions = BTreeMap::new();
        for action in self.actions() {
            action_descriptions.insert(action.name(), action.full_description());
        }

        let mut event_descriptions = BTreeMap::new();
        for event in self.events() {
            event_descriptions.insert(event.name(), event.full_description()?);
        }

        Ok(self.description().into_full_description(
            self.id(),
            property_descriptions,
            action_descriptions,
            event_descriptions,
        ))
    }
}

/// A trait used to build a [Device] around a data struct and a [device handle][DeviceHandle].
///
/// When you use the [device][macro@crate::device] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::{BuiltDevice, DeviceBuilder}};
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
/// impl BuiltDevice for BuiltExampleDevice {
///     // ...
///   # fn device_handle(&self) -> &DeviceHandle {
///   #     &self.device_handle
///   # }
///   # fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///   #     &mut self.device_handle
///   # }
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {}
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
/// impl DeviceBuilder for ExampleDevice {
///     type BuiltDevice = BuiltExampleDevice;
///     fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
///         BuiltExampleDevice {
///             data,
///             device_handle,
///         }
///     }
/// }
/// ```
pub trait DeviceBuilder: DeviceStructure {
    /// Type of [Device] to build.
    type BuiltDevice: Device;

    /// Build the [device][Device] from a data struct and an [device handle][DeviceHandle].
    fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        action::{tests::MockAction, NoInput},
        actions,
        device::{tests::BuiltMockDevice, DeviceBuilder},
        event::{tests::MockEvent, NoData},
        events, properties,
        property::tests::MockProperty,
        Actions, DeviceDescription, DeviceHandle, DeviceStructure, Events, Properties,
    };

    pub struct MockDevice {
        device_id: String,
    }

    impl MockDevice {
        pub fn new(device_id: String) -> Self {
            Self { device_id }
        }

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
    }

    impl DeviceStructure for MockDevice {
        fn id(&self) -> String {
            self.device_id.clone()
        }

        fn description(&self) -> DeviceDescription {
            DeviceDescription::default()
        }

        fn properties(&self) -> Properties {
            properties![
                MockProperty::<bool>::new(MockDevice::PROPERTY_BOOL.to_owned()),
                MockProperty::<u8>::new(MockDevice::PROPERTY_U8.to_owned()),
                MockProperty::<i32>::new(MockDevice::PROPERTY_I32.to_owned()),
                MockProperty::<f32>::new(MockDevice::PROPERTY_F32.to_owned()),
                MockProperty::<Option<i32>>::new(MockDevice::PROPERTY_OPTI32.to_owned()),
                MockProperty::<String>::new(MockDevice::PROPERTY_STRING.to_owned())
            ]
        }

        fn actions(&self) -> Actions {
            actions![
                MockAction::<NoInput>::new(MockDevice::ACTION_NOINPUT.to_owned()),
                MockAction::<bool>::new(MockDevice::ACTION_BOOL.to_owned()),
                MockAction::<u8>::new(MockDevice::ACTION_U8.to_owned()),
                MockAction::<i32>::new(MockDevice::ACTION_I32.to_owned()),
                MockAction::<f32>::new(MockDevice::ACTION_F32.to_owned()),
                MockAction::<Option<i32>>::new(MockDevice::ACTION_OPTI32.to_owned()),
                MockAction::<String>::new(MockDevice::ACTION_STRING.to_owned())
            ]
        }

        fn events(&self) -> Events {
            events![MockEvent::<NoData>::new(
                MockDevice::EVENT_NODATA.to_owned()
            )]
        }
    }

    impl DeviceBuilder for MockDevice {
        type BuiltDevice = BuiltMockDevice;
        fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
            BuiltMockDevice::new(data, device_handle)
        }
    }
}
