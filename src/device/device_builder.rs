/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    actions, api_error::ApiError, events, properties, Actions, Device, DeviceDescription,
    DeviceHandle, Events, Properties,
};

use std::collections::BTreeMap;

use webthings_gateway_ipc_types::Device as FullDeviceDescription;

/// A trait used to specify the structure of a WoT device.
///
/// Builds a [Device] instance. Created through an [adapter][crate::Adapter].
///
/// # Examples
/// ```
/// # #[macro_use]
/// # extern crate gateway_addon_rust;
/// # use gateway_addon_rust::{prelude::*, example::{ExampleDevice, ExamplePropertyBuilder, ExampleEvent, ExampleAction}};
/// # fn main() {}
/// // ...
/// pub struct ExampleDeviceBuilder();
///
/// impl DeviceBuilder for ExampleDeviceBuilder {
///     type Device = ExampleDevice;
///
///     fn id(&self) -> String {
///         "example-device".to_owned()
///     }
///
///     fn description(&self) -> DeviceDescription {
///         DeviceDescription::default()
///     }
///
///     fn properties(&self) -> Properties {
///         properties![ExamplePropertyBuilder::new()]
///     }
///
///     fn actions(&self) -> Actions {
///         actions![ExampleAction::new()]
///     }
///
///     fn events(&self) -> Events {
///         events![ExampleEvent::new()]
///     }
///
///     fn build(self, device_handle: DeviceHandle) -> Self::Device {
///         ExampleDevice::new(device_handle)
///     }
/// }
/// ```
pub trait DeviceBuilder: Send + Sync + 'static {
    /// Type of [device][Device] this builds.
    type Device: Device;

    /// ID of the device.
    fn id(&self) -> String;

    /// [WoT description][DeviceDescription] of the device.
    fn description(&self) -> DeviceDescription;

    /// A list of [properties][crate::PropertyBuilder] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [PropertyBuilderBase][crate::property::PropertyBuilderBase].
    /// You can use the convenienve macro [properties!][crate::properties] to create this list [PropertyBuilder][crate::PropertyBuilder]s.
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

    /// Build a new instance of this device using the given [device handle][DeviceHandle].
    fn build(self, device_handle: DeviceHandle) -> Self::Device;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullDeviceDescription, ApiError> {
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

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        action::{tests::MockAction, NoInput},
        actions,
        device::tests::MockDevice,
        event::{tests::MockEvent, NoData},
        events, properties,
        property::tests::MockPropertyBuilder,
        Actions, DeviceBuilder, DeviceDescription, DeviceHandle, Events, Properties,
    };

    pub struct MockDeviceBuilder {
        device_id: String,
    }

    impl MockDeviceBuilder {
        pub fn new(device_id: String) -> Self {
            Self { device_id }
        }
    }

    impl DeviceBuilder for MockDeviceBuilder {
        type Device = MockDevice;

        fn id(&self) -> String {
            self.device_id.clone()
        }

        fn description(&self) -> DeviceDescription {
            DeviceDescription::default()
        }

        fn properties(&self) -> Properties {
            properties![
                MockPropertyBuilder::<bool>::new(MockDevice::PROPERTY_BOOL.to_owned()),
                MockPropertyBuilder::<u8>::new(MockDevice::PROPERTY_U8.to_owned()),
                MockPropertyBuilder::<i32>::new(MockDevice::PROPERTY_I32.to_owned()),
                MockPropertyBuilder::<f32>::new(MockDevice::PROPERTY_F32.to_owned()),
                MockPropertyBuilder::<Option<i32>>::new(MockDevice::PROPERTY_OPTI32.to_owned()),
                MockPropertyBuilder::<String>::new(MockDevice::PROPERTY_STRING.to_owned())
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

        fn build(self, device_handle: DeviceHandle) -> Self::Device {
            MockDevice::new(device_handle)
        }
    }
}
