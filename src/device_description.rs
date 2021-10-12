/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use webthings_gateway_ipc_types::{DevicePin, Link};

pub struct DeviceDescription {
    pub at_context: Option<String>,
    pub at_type: Option<Vec<String>>,
    pub base_href: Option<String>,
    pub credentials_required: Option<bool>,
    pub description: Option<String>,
    pub links: Option<Vec<Link>>,
    pub pin: Option<DevicePin>,
    pub title: Option<String>,
}

#[derive(Debug)]
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

pub trait DeviceDescriptionBuilder {
    fn at_context<S: Into<String>>(self, at_context: S) -> Self;
    fn at_types(self, at_types: Vec<AtType>) -> Self;
    fn at_type(self, at_type: AtType) -> Self;
    fn base_href<S: Into<String>>(self, base_href: S) -> Self;
    fn credentials_required(self, credentials_required: bool) -> Self;
    fn description<S: Into<String>>(self, description: S) -> Self;
    fn links(self, links: Vec<Link>) -> Self;
    fn link(self, link: Link) -> Self;
    fn pin(self, pin: DevicePin) -> Self;
    fn title<S: Into<String>>(self, title: S) -> Self;
    fn default() -> Self;
}

impl DeviceDescriptionBuilder for DeviceDescription {
    fn at_context<S: Into<String>>(mut self, at_context: S) -> Self {
        self.at_context = Some(at_context.into());
        self
    }

    fn at_types(mut self, at_types: Vec<AtType>) -> Self {
        self.at_type = Some(
            at_types
                .into_iter()
                .map(|at_type| at_type.to_string())
                .collect(),
        );
        self
    }

    fn at_type(mut self, at_type: AtType) -> Self {
        match self.at_type {
            None => self.at_type = Some(vec![at_type.to_string()]),
            Some(ref mut at_types) => at_types.push(at_type.to_string()),
        };
        self
    }

    fn base_href<S: Into<String>>(mut self, base_href: S) -> Self {
        self.base_href = Some(base_href.into());
        self
    }

    fn credentials_required(mut self, credentials_required: bool) -> Self {
        self.credentials_required = Some(credentials_required);
        self
    }

    fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    fn pin(mut self, pin: DevicePin) -> Self {
        self.pin = Some(pin);
        self
    }

    fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    fn default() -> Self {
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
}
