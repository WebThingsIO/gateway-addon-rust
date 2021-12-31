/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use std::collections::BTreeMap;
use webthings_gateway_ipc_types::{
    Action as FullActionDescription, Device as FullDeviceDescription, DevicePin,
    Event as FullEventDescription, Link, Property as FullPropertyDescription,
};

/// A struct which represents a WoT [device description][webthings_gateway_ipc_types::Device].
///
/// This is used by [DeviceStructure][crate::DeviceStructure].
///
/// Use the provided builder methods instead of directly writing to the struct fields.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::AtType};
/// # let _ =
/// DeviceDescription::default()
///     .title("Foo device")
///     .at_types(vec![AtType::Light, AtType::OnOffSwitch])
///     .credentials_required(true)
/// # ;
/// ```
#[derive(Clone)]
pub struct DeviceDescription {
    pub at_context: Option<String>,
    pub at_type: Option<Vec<AtType>>,
    pub base_href: Option<String>,
    pub credentials_required: Option<bool>,
    pub description: Option<String>,
    pub links: Option<Vec<Link>>,
    pub pin: Option<DevicePin>,
    pub title: Option<String>,
}

/// Possible values of `@type` for a [device][DeviceDescription].
#[derive(Debug, Clone)]
pub enum AtType {
    Alarm,
    AirQualitySensor,
    BarometricPressureSensor,
    BinarySensor,
    Camera,
    ColorControl,
    ColorSensor,
    DoorSensor,
    EnergyMonitor,
    HumiditySensor,
    LeakSensor,
    Light,
    Lock,
    MotionSensor,
    MultiLevelSensor,
    MultiLevelSwitch,
    OnOffSwitch,
    PushButton,
    SmartPlug,
    SmokeSensor,
    TemperatureSensor,
    Thermostat,
    VideoCamera,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// # Builder methods
impl DeviceDescription {
    /// Build an empty [DeviceDescription].
    pub fn default() -> Self {
        Self {
            at_context: None,
            at_type: None,
            base_href: None,
            credentials_required: None,
            description: None,
            links: None,
            pin: None,
            title: None,
        }
    }

    /// Set `@context`.
    pub fn at_context(mut self, at_context: impl Into<String>) -> Self {
        self.at_context = Some(at_context.into());
        self
    }

    /// Set `@type`.
    pub fn at_types(mut self, at_types: Vec<AtType>) -> Self {
        self.at_type = Some(at_types);
        self
    }

    /// Add a single [AtType] to `@type`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::device::{DeviceDescription, AtType};
    /// # let _ =
    /// DeviceDescription::default()
    ///     .at_type(AtType::Light)
    ///     .at_type(AtType::OnOffSwitch)
    /// # ;
    /// ```
    pub fn at_type(mut self, at_type: AtType) -> Self {
        match self.at_type {
            None => self.at_type = Some(vec![at_type]),
            Some(ref mut at_types) => at_types.push(at_type),
        };
        self
    }

    /// Set `baseHref`.
    pub fn base_href(mut self, base_href: impl Into<String>) -> Self {
        self.base_href = Some(base_href.into());
        self
    }

    /// Set `credentialsRequired`.
    pub fn credentials_required(mut self, credentials_required: bool) -> Self {
        self.credentials_required = Some(credentials_required);
        self
    }

    /// Set `description`.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set `links`.
    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    /// Add a single link to `links`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::device::DeviceDescription;
    /// # use webthings_gateway_ipc_types::Link;
    /// # let _ =
    /// DeviceDescription::default()
    ///     .link(Link {
    ///         href: "https://www.rust-lang.org/".to_owned(),
    ///         media_type: None,
    ///         rel: None,
    ///     })
    ///     .link(Link {
    ///         href: "https://www.reddit.com/".to_owned(),
    ///         media_type: None,
    ///         rel: None,
    ///     })
    /// # ;
    /// ```
    pub fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    /// Set `pin`.
    pub fn pin(mut self, pin: DevicePin) -> Self {
        self.pin = Some(pin);
        self
    }

    /// Set `title`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[doc(hidden)]
    pub fn into_full_description(
        self,
        id: String,
        property_descriptions: BTreeMap<String, FullPropertyDescription>,
        action_descriptions: BTreeMap<String, FullActionDescription>,
        event_descriptions: BTreeMap<String, FullEventDescription>,
    ) -> FullDeviceDescription {
        FullDeviceDescription {
            at_context: self.at_context,
            at_type: self
                .at_type
                .map(|v| v.into_iter().map(|t| t.to_string()).collect()),
            id,
            title: self.title,
            description: self.description,
            properties: Some(property_descriptions),
            actions: Some(action_descriptions),
            events: Some(event_descriptions),
            links: self.links,
            base_href: self.base_href,
            pin: self.pin,
            credentials_required: self.credentials_required,
        }
    }
}
