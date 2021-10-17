/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use serde_json::Value;
pub use webthings_gateway_ipc_types::{Link, Property as FullPropertyDescription};

pub struct PropertyDescription {
    pub at_type: Option<String>,
    pub description: Option<String>,
    pub enum_: Option<Vec<Value>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub read_only: Option<bool>,
    pub title: Option<String>,
    pub type_: String,
    pub unit: Option<String>,
    pub value: Option<Value>,
    pub visible: Option<bool>,
}

pub enum Type {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Null => "null",
            Type::Boolean => "boolean",
            Type::Integer => "integer",
            Type::Number => "number",
            Type::String => "string",
        }
        .to_owned()
    }
}

#[derive(Debug)]
pub enum AtType {
    AlarmProperty,
    BarometricPressureProperty,
    BooleanProperty,
    BrightnessProperty,
    ColorModeProperty,
    ColorProperty,
    ColorTemperatureProperty,
    ConcentrationProperty,
    CurrentProperty,
    DensityProperty,
    FrequencyProperty,
    HeatingCoolingProperty,
    HumidityProperty,
    ImageProperty,
    InstantaneousPowerFactorProperty,
    InstantaneousPowerProperty,
    LeakProperty,
    LevelProperty,
    LockedProperty,
    MotionProperty,
    OnOffProperty,
    OpenProperty,
    PushedProperty,
    SmokeProperty,
    TargetTemperatureProperty,
    TemperatureProperty,
    ThermostatModeProperty,
    VideoProperty,
    VoltageProperty,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl PropertyDescription {
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn enum_(mut self, enum_: Vec<Value>) -> Self {
        self.enum_ = Some(enum_);
        self
    }

    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    pub fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    pub fn maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }

    pub fn minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }

    pub fn multiple_of(mut self, multiple_of: f64) -> Self {
        self.multiple_of = Some(multiple_of);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn type_(mut self, type_: Type) -> Self {
        self.type_ = type_.to_string();
        self
    }

    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn value<V: Into<Value>>(mut self, value: V) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn default() -> Self {
        Self {
            at_type: None,
            description: None,
            enum_: None,
            links: None,
            maximum: None,
            minimum: None,
            multiple_of: None,
            read_only: None,
            title: None,
            type_: Type::Null.to_string(),
            unit: None,
            value: None,
            visible: None,
        }
    }
}
