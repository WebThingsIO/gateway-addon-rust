/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{type_::Type, ApiError};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Link, Property as FullPropertyDescription};

#[derive(Clone)]
pub struct PropertyDescription<T: Value> {
    pub at_type: Option<AtType>,
    pub description: Option<String>,
    pub enum_: Option<Vec<T>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub read_only: Option<bool>,
    pub title: Option<String>,
    pub type_: Type,
    pub unit: Option<String>,
    pub value: T,
    pub visible: Option<bool>,
    _value: PhantomData<T>,
}

pub trait Value: Clone + Default + Send + Sync + 'static {
    fn type_() -> Type {
        Type::Object
    }

    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError>;

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError>;
}

pub trait SimpleValue:
    Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static
{
    fn type_() -> Type {
        Type::Object
    }

    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(Some(
            serde_json::to_value(value).map_err(ApiError::Serialization)?,
        ))
    }

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        serde_json::from_value(value.ok_or_else(|| {
            ApiError::Serialization(<serde_json::Error as serde::de::Error>::custom(
                "Expected Some, found None",
            ))
        })?)
        .map_err(ApiError::Serialization)
    }
}

impl<T: SimpleValue> Value for T {
    fn type_() -> Type {
        <T as SimpleValue>::type_()
    }

    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        <T as SimpleValue>::description(description)
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        <T as SimpleValue>::serialize(value)
    }

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        <T as SimpleValue>::deserialize(value)
    }
}

impl SimpleValue for i8 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for i16 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for i32 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for u8 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for u16 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for u32 {
    fn type_() -> Type {
        Type::Integer
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for f32 {
    fn type_() -> Type {
        Type::Number
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for f64 {
    fn type_() -> Type {
        Type::Number
    }
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleValue for bool {
    fn type_() -> Type {
        Type::Boolean
    }
}

impl SimpleValue for String {
    fn type_() -> Type {
        Type::String
    }
}

impl SimpleValue for serde_json::Value {}

impl<T: Value> Value for Vec<T> {
    fn type_() -> Type {
        Type::Array
    }

    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        let mut v = Vec::new();
        for t in value {
            v.push(T::serialize(t)?.ok_or_else(|| {
                ApiError::Serialization(<serde_json::Error as serde::ser::Error>::custom(
                    "Expected Some, found None",
                ))
            })?);
        }
        Ok(Some(serde_json::Value::Array(v)))
    }

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        if let Some(value) = value {
            if let serde_json::Value::Array(value) = value {
                let mut v = Vec::new();
                for e in value {
                    v.push(T::deserialize(Some(e))?);
                }
                Ok(v)
            } else {
                Err(ApiError::Serialization(
                    <serde_json::Error as serde::de::Error>::custom("Expected Array"),
                ))
            }
        } else {
            Err(ApiError::Serialization(
                <serde_json::Error as serde::de::Error>::custom("Expected Some, found None"),
            ))
        }
    }
}

impl<T: Value> Value for Option<T> {
    fn type_() -> Type {
        T::type_()
    }

    fn description(mut description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        let t_description = T::description(PropertyDescription::default());
        description.at_type = t_description.at_type;
        description.description = t_description.description;
        description.enum_ = t_description
            .enum_
            .map(|e| e.into_iter().map(|t| Some(t)).collect());
        description.links = t_description.links;
        description.maximum = t_description.maximum;
        description.minimum = t_description.minimum;
        description.multiple_of = t_description.multiple_of;
        description.read_only = t_description.read_only;
        description.title = t_description.title;
        description.unit = t_description.unit;
        description.visible = t_description.visible;
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(if let Some(value) = value {
            T::serialize(value)?
        } else {
            None
        })
    }

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        Ok(if let Some(value) = value {
            match value {
                serde_json::Value::Null => None,
                _ => Some(T::deserialize(Some(value))?),
            }
        } else {
            None
        })
    }
}

#[derive(Debug, Clone)]
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
    pub fn default() -> Self {
        T::description(Self {
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
            value: T::default(),
            visible: None,
            _value: PhantomData,
        })
    }

    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type);
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn enum_(mut self, enum_: Vec<T>) -> Self {
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

    pub fn maximum<F: Into<f64>>(mut self, maximum: F) -> Self {
        self.maximum = Some(maximum.into());
        self
    }

    pub fn minimum<F: Into<f64>>(mut self, minimum: F) -> Self {
        self.minimum = Some(minimum.into());
        self
    }

    pub fn multiple_of<F: Into<f64>>(mut self, multiple_of: F) -> Self {
        self.multiple_of = Some(multiple_of.into());
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
        self.type_ = type_;
        self
    }

    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    #[doc(hidden)]
    pub fn into_full_description(self, name: String) -> Result<FullPropertyDescription, ApiError> {
        let enum_ = if let Some(enum_) = self.enum_ {
            let mut v = Vec::new();
            for e in enum_ {
                v.push(T::serialize(e)?.ok_or_else(|| {
                    ApiError::Serialization(<serde_json::Error as serde::ser::Error>::custom(
                        "Expected Some, found None",
                    ))
                })?);
            }
            Some(v)
        } else {
            None
        };
        Ok(FullPropertyDescription {
            at_type: self.at_type.map(|t| t.to_string()),
            description: self.description,
            enum_,
            links: self.links,
            maximum: self.maximum,
            minimum: self.minimum,
            multiple_of: self.multiple_of,
            read_only: self.read_only,
            title: self.title,
            type_: self.type_.to_string(),
            unit: self.unit,
            value: T::serialize(self.value)?,
            visible: self.visible,
            name: Some(name),
        })
    }
}
