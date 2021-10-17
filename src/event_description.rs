/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use std::marker::PhantomData;

use serde::Serialize;
use serde_json::Value;
use webthings_gateway_ipc_types::Link;

use crate::api_error::ApiError;

pub trait Data: Clone + Send + Sync + Serialize + Sized {
    fn type_() -> Option<String>;
    fn description(_description: &mut EventDescription<Self>) {}
    fn serialize(self) -> Result<Option<Value>, ApiError> {
        Ok(Some(
            serde_json::to_value(self).map_err(ApiError::Serialization)?,
        ))
    }
}

#[derive(Clone, Serialize)]
pub struct NoData;

impl Data for NoData {
    fn type_() -> Option<String> {
        None
    }
    fn serialize(self) -> Result<Option<Value>, ApiError> {
        Ok(None)
    }
}

#[derive(Clone, Serialize)]
pub struct Null;

impl Data for Null {
    fn type_() -> Option<String> {
        Some("null".to_owned())
    }
}

impl Data for i8 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for i16 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for i32 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for u8 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for u16 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for u32 {
    fn type_() -> Option<String> {
        Some("integer".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for f32 {
    fn type_() -> Option<String> {
        Some("number".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN.into());
        description.maximum = Some(Self::MAX.into());
    }
}

impl Data for f64 {
    fn type_() -> Option<String> {
        Some("number".to_owned())
    }
    fn description(description: &mut EventDescription<Self>) {
        description.minimum = Some(Self::MIN);
        description.maximum = Some(Self::MAX);
    }
}

impl Data for bool {
    fn type_() -> Option<String> {
        Some("boolean".to_owned())
    }
}

impl Data for String {
    fn type_() -> Option<String> {
        Some("string".to_owned())
    }
}

impl Data for Value {
    fn type_() -> Option<String> {
        Some("object".to_owned())
    }
}

impl<T: Data> Data for Vec<T> {
    fn type_() -> Option<String> {
        Some("array".to_owned())
    }
}

#[derive(Debug)]
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

pub struct EventDescription<T: Data> {
    pub at_type: Option<String>,
    pub description: Option<String>,
    pub enum_: Option<Vec<Value>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub title: Option<String>,
    pub type_: Option<String>,
    pub unit: Option<String>,
    _data: PhantomData<T>,
}

impl<T: Data> EventDescription<T> {
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

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn type_<S: Into<String>>(mut self, type_: S) -> Self {
        self.type_ = Some(type_.into());
        self
    }

    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
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
            title: None,
            type_: T::type_(),
            unit: None,
            _data: PhantomData,
        };
        T::description(&mut description);
        description
    }
}
