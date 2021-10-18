/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use webthings_gateway_ipc_types::{Link, Property as FullPropertyDescription};

pub struct PropertyDescription<T: Value> {
    pub at_type: Option<String>,
    pub description: Option<String>,
    pub enum_: Option<Vec<serde_json::Value>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub read_only: Option<bool>,
    pub title: Option<String>,
    pub type_: String,
    pub unit: Option<String>,
    pub value: Option<serde_json::Value>,
    pub visible: Option<bool>,
    _value: PhantomData<T>,
}

pub trait Value: Clone + Send + Sync + Serialize + DeserializeOwned + Sized {
    fn type_() -> String;
    fn description(_description: &mut PropertyDescription<Self>) {}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Null;

impl Value for Null {
    fn type_() -> String {
        "null".to_owned()
    }
}

impl Value for i8 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for i16 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for i32 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for u8 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for u16 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for u32 {
    fn type_() -> String {
        "integer".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for f32 {
    fn type_() -> String {
        "number".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Value for f64 {
    fn type_() -> String {
        "number".to_owned()
    }
    fn description(description: &mut PropertyDescription<Self>) {
        description.minimum = Some(Self::MIN);
        description.maximum = Some(Self::MAX);
    }
}

impl Value for bool {
    fn type_() -> String {
        "boolean".to_owned()
    }
}

impl Value for String {
    fn type_() -> String {
        "string".to_owned()
    }
}

impl Value for serde_json::Value {
    fn type_() -> String {
        "object".to_owned()
    }
}

impl<T: Value> Value for Vec<T> {
    fn type_() -> String {
        "array".to_owned()
    }
}

impl<T: Value> Value for Option<T> {
    fn type_() -> String {
        T::type_()
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

impl<T: Value> PropertyDescription<T> {
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn enum_(mut self, enum_: Vec<serde_json::Value>) -> Self {
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

    pub fn type_<S: Into<String>>(mut self, type_: S) -> Self {
        self.type_ = type_.into();
        self
    }

    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn value(mut self, value: T) -> Self {
        self.value = Some(serde_json::to_value(value).unwrap());
        self
    }

    pub fn value_<V: Into<serde_json::Value>>(mut self, value: V) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn default() -> Self {
        let mut description = Self {
            at_type: None,
            description: None,
            enum_: None,
            links: None,
            maximum: None,
            minimum: None,
            multiple_of: None,
            read_only: None,
            title: None,
            type_: T::type_(),
            unit: None,
            value: None,
            visible: None,
            _value: PhantomData,
        };
        T::description(&mut description);
        description
    }
}
