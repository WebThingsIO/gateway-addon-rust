/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for working with WoT datatypes.

use crate::{action::Input, event_description::Data, property_description::Value, ApiError};
use serde_json::json;

/// An enum of all WoT datatypes.
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

/// An equivalent of the WoT [type][Type] null.
#[derive(Clone, Default, PartialEq, Debug)]
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
            Ok(Null)
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

#[cfg(test)]
mod tests {
    use crate::{action, event, property, type_::Null};
    use serde_json::json;

    #[tokio::test]
    async fn test_null_value_deserialize() {
        assert_eq!(
            <Null as property::Value>::deserialize(Some(json!(null))).unwrap(),
            Null
        );
        assert_eq!(<Null as property::Value>::deserialize(None).unwrap(), Null);
        assert!(<Null as property::Value>::deserialize(Some(json!(42))).is_err());
    }

    #[tokio::test]
    async fn test_null_value_serialize() {
        assert_eq!(
            <Null as property::Value>::serialize(Null).unwrap(),
            Some(json!(null))
        );
    }

    #[tokio::test]
    async fn test_null_input_deserialize() {
        assert_eq!(
            <Null as action::Input>::deserialize(json!(null)).unwrap(),
            Null
        );
        assert!(<Null as action::Input>::deserialize(json!(42)).is_err());
    }

    #[tokio::test]
    async fn test_null_data_serialize() {
        assert_eq!(
            <Null as event::Data>::serialize(Null).unwrap(),
            Some(json!(null))
        );
    }
}
