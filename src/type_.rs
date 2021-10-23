/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    action_description::Input, event_description::Data, property_description::Value, ApiError,
};
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Type {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Null => "null",
            Type::Boolean => "boolean",
            Type::Integer => "integer",
            Type::Number => "number",
            Type::String => "string",
            Type::Array => "array",
            Type::Object => "object",
        }
        .to_owned()
    }
}

#[derive(Clone, Default)]
pub struct Null;

impl Value for Null {
    fn type_() -> Type {
        Type::Null
    }

    fn serialize(_value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(Some(json!(null)))
    }

    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        if let Some(value) = value {
            match value {
                serde_json::Value::Null => Ok(Null),
                _ => Err(ApiError::Serialization(
                    <serde_json::Error as serde::de::Error>::custom("Expected Null"),
                )),
            }
        } else {
            Err(ApiError::Serialization(
                <serde_json::Error as serde::de::Error>::custom("Expected Some, found None"),
            ))
        }
    }
}

impl Data for Null {
    fn type_() -> Option<Type> {
        Some(Type::Null)
    }

    fn serialize(_value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(Some(json!(null)))
    }
}

impl Input for Null {
    fn input() -> Option<serde_json::Value> {
        Some(json!({"type": "null"}))
    }
    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        match value {
            serde_json::Value::Null => Ok(Null),
            _ => Err(ApiError::Serialization(
                <serde_json::Error as serde::de::Error>::custom("Expected Null"),
            )),
        }
    }
}
