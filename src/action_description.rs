/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::api_error::ApiError;
use schemars::{schema_for, JsonSchema};
use serde::de::{DeserializeOwned, Error};
use serde_json::json;
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Action as FullActionDescription, Link};

#[derive(Clone)]
pub struct ActionDescription<T: Input> {
    pub at_type: Option<AtType>,
    pub description: Option<String>,
    pub input: Option<serde_json::Value>,
    pub links: Option<Vec<Link>>,
    pub title: Option<String>,
    pub _input: PhantomData<T>,
}

pub trait Input: Clone + Send + Sync + 'static {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
        }))
    }

    fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
        description
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError>;
}

pub trait SimpleInput: DeserializeOwned + JsonSchema + Clone + Send + Sync + 'static {
    fn input() -> Option<serde_json::Value> {
        if let Ok(schema) = serde_json::to_value(&schema_for!(Self)) {
            Some(schema)
        } else {
            None
        }
    }

    fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
        description
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        serde_json::from_value(value).map_err(ApiError::Serialization)
    }
}

impl<T: SimpleInput> Input for T {
    fn input() -> Option<serde_json::Value> {
        <T as SimpleInput>::input()
    }

    fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
        <T as SimpleInput>::description(description)
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        <T as SimpleInput>::deserialize(value)
    }
}

#[derive(Clone)]
pub struct NoInput;

impl Input for NoInput {
    fn input() -> Option<serde_json::Value> {
        None
    }

    fn deserialize(_value: serde_json::Value) -> Result<Self, ApiError> {
        Ok(NoInput)
    }
}

impl SimpleInput for i8 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for i16 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for i32 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for u8 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for u16 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for u32 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for f32 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "number",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for f64 {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "number",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl SimpleInput for bool {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "boolean",
        }))
    }
}

impl SimpleInput for String {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "string",
        }))
    }
}

impl SimpleInput for serde_json::Value {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
        }))
    }
}

impl<T: Input> Input for Vec<T> {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "array",
            "items": T::input()
        }))
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        if let serde_json::Value::Array(v) = value {
            let mut w = Vec::new();
            for e in v {
                w.push(T::deserialize(e)?);
            }
            Ok(w)
        } else {
            Err(ApiError::Serialization(serde_json::Error::custom(
                "Expected Array",
            )))
        }
    }
}

impl<T: Input> Input for Option<T> {
    fn input() -> Option<serde_json::Value> {
        T::input()
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        Ok(match value {
            serde_json::Value::Null => None,
            _ => Some(T::deserialize(value)?),
        })
    }
}

#[derive(Debug, Clone)]
pub enum AtType {
    FadeAction,
    LockAction,
    ToggleAction,
    UnlockAction,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl<T: Input> ActionDescription<T> {
    pub fn default() -> Self {
        Self {
            at_type: None,
            description: None,
            links: None,
            title: None,
            input: T::input(),
            _input: PhantomData,
        }
    }

    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type);
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
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

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    #[doc(hidden)]
    pub fn into_full_description(self) -> FullActionDescription {
        FullActionDescription {
            at_type: self.at_type.map(|t| t.to_string()),
            description: self.description,
            input: self.input,
            links: self.links,
            title: self.title,
        }
    }
}
