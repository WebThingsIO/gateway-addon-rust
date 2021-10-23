/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{api_error::ApiError, type_::Type};
use serde::{ser::Error, Serialize};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Event as FullEventDescription, Link};

#[derive(Clone)]
pub struct EventDescription<T: Data> {
    pub at_type: Option<AtType>,
    pub description: Option<String>,
    pub enum_: Option<Vec<T>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub title: Option<String>,
    pub type_: Option<Type>,
    pub unit: Option<String>,
    _data: PhantomData<T>,
}

pub trait Data: Clone + Send + Sync + 'static {
    fn type_() -> Option<Type> {
        Some(Type::Object)
    }

    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError>;
}

pub trait SimpleData: Serialize + Clone + Send + Sync + 'static {
    fn type_() -> Option<Type> {
        Some(Type::Object)
    }

    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(Some(
            serde_json::to_value(value).map_err(ApiError::Serialization)?,
        ))
    }
}

impl<T: SimpleData> Data for T {
    fn type_() -> Option<Type> {
        <T as SimpleData>::type_()
    }

    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        <T as SimpleData>::description(description)
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        <T as SimpleData>::serialize(value)
    }
}

#[derive(Clone, Serialize)]
pub struct NoData;

impl Data for NoData {
    fn type_() -> Option<Type> {
        None
    }

    fn serialize(_value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(None)
    }
}

impl SimpleData for i8 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for i16 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for i32 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for u8 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for u16 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for u32 {
    fn type_() -> Option<Type> {
        Some(Type::Integer)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for f32 {
    fn type_() -> Option<Type> {
        Some(Type::Number)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for f64 {
    fn type_() -> Option<Type> {
        Some(Type::Number)
    }
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description.minimum(Self::MIN).maximum(Self::MAX)
    }
}

impl SimpleData for bool {
    fn type_() -> Option<Type> {
        Some(Type::Boolean)
    }
}

impl SimpleData for String {
    fn type_() -> Option<Type> {
        Some(Type::String)
    }
}

impl SimpleData for serde_json::Value {}

impl<T: Data> Data for Vec<T> {
    fn type_() -> Option<Type> {
        Some(Type::Array)
    }

    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        let mut v = Vec::new();
        for e in value {
            v.push(T::serialize(e)?.ok_or_else(|| {
                ApiError::Serialization(serde_json::Error::custom("Expected Some, found None"))
            })?);
        }
        Ok(Some(serde_json::Value::Array(v)))
    }
}

impl<T: Data> Data for Option<T> {
    fn type_() -> Option<Type> {
        T::type_()
    }

    fn description(mut description: EventDescription<Self>) -> EventDescription<Self> {
        let t_description = T::description(EventDescription::default());
        description.at_type = t_description.at_type;
        description.description = t_description.description;
        description.enum_ = t_description
            .enum_
            .map(|e| e.into_iter().map(Some).collect());
        description.links = t_description.links;
        description.maximum = t_description.maximum;
        description.minimum = t_description.minimum;
        description.multiple_of = t_description.multiple_of;
        description.title = t_description.title;
        description.unit = t_description.unit;
        description
    }

    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(if let Some(value) = value {
            Some(T::serialize(value)?.ok_or_else(|| {
                ApiError::Serialization(serde_json::Error::custom("Expected Some, found None"))
            })?)
        } else {
            None
        })
    }
}

#[derive(Debug, Clone)]
pub enum AtType {
    AlarmEvent,
    DoublePressedEvent,
    LongPressedEvent,
    OverheatedEvent,
    PressedEvent,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl<T: Data> EventDescription<T> {
    pub fn default() -> Self {
        let description = Self {
            at_type: None,
            description: None,
            enum_: None,
            links: None,
            maximum: None,
            minimum: None,
            multiple_of: None,
            title: None,
            type_: T::type_(),
            unit: None,
            _data: PhantomData,
        };
        T::description(description)
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

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn type_(mut self, type_: Type) -> Self {
        self.type_ = Some(type_);
        self
    }

    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
        self
    }

    #[doc(hidden)]
    pub fn into_full_description(self, name: String) -> Result<FullEventDescription, ApiError> {
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
        Ok(FullEventDescription {
            at_type: self.at_type.map(|t| t.to_string()),
            description: self.description,
            enum_,
            links: self.links,
            maximum: self.maximum,
            minimum: self.minimum,
            multiple_of: self.multiple_of,
            name: Some(name),
            title: self.title,
            type_: self.type_.map(|t| t.to_string()),
            unit: self.unit,
        })
    }
}
