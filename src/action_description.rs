/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::api_error::ApiError;
use schemars::{schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::Link;

pub struct ActionDescription<I: JsonSchema> {
    pub at_type: Option<String>,
    pub description: Option<String>,
    pub input: Option<Value>,
    pub links: Option<Vec<Link>>,
    pub title: Option<String>,
    pub _input: PhantomData<I>,
}

pub trait Input: Clone + Send + Sync + DeserializeOwned + JsonSchema + Sized {
    fn input() -> Option<Value> {
        if let Ok(schema) = serde_json::to_value(&schema_for!(Self)) {
            Some(schema)
        } else {
            None
        }
    }
    fn deserialize(value: Value) -> Result<Self, ApiError> {
        serde_json::from_value(value).map_err(ApiError::Serialization)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub struct NoInput;

impl Input for NoInput {
    fn input() -> Option<Value> {
        None
    }

    fn deserialize(_value: Value) -> Result<Self, ApiError> {
        Ok(NoInput)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub struct Null;

impl Input for Null {
    fn input() -> Option<Value> {
        Some(json!({"type": "null"}))
    }
}

impl Input for i8 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for i16 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for i32 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for u8 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for u16 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for u32 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "integer",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for f32 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "number",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for f64 {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "number",
            "minimum": Self::MIN,
            "maximum": Self::MAX,
        }))
    }
}

impl Input for bool {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "boolean",
        }))
    }
}

impl Input for String {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "string",
        }))
    }
}

impl Input for Value {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "object",
        }))
    }
}

impl<T: Input> Input for Vec<T> {
    fn input() -> Option<Value> {
        Some(json!({
            "type": "array",
            "items": T::input()
        }))
    }
}

impl<T: Input> Input for Option<T> {
    fn input() -> Option<Value> {
        T::input()
    }
}

#[derive(Debug)]
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
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn input(mut self, input: Value) -> Self {
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
}
