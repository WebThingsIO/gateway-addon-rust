/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{api_error::ApiError, type_::Type, PropertyDescription};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;

/// A trait which converts between Rust types and WoT [types][Type].
///
/// Already implemented for common Rust types. You may want to implement [SimpleValue] instead of this.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, type_::Type, property::{AtType, Value}, api_error::ApiError};
/// # use serde::de::Error;
/// #[derive(Clone, Default)]
/// struct Level(i32);
///
/// impl Value for Level {
///     fn type_() -> Type {
///         Type::Integer
///     }
///     fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
///         description.at_type(AtType::LevelProperty)
///     }
///     fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
///         Ok(Some(
///             serde_json::to_value(value.0).map_err(ApiError::Serialization)?,
///         ))
///     }
///     fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
///         Ok(Self(
///             serde_json::from_value(value.ok_or_else(|| {
///                 ApiError::Serialization(serde_json::Error::custom("Expected Some, found None"))
///             })?)
///             .map_err(ApiError::Serialization)?,
///         ))
///     }
/// }
/// ```
pub trait Value: Clone + Default + Send + Sync + 'static {
    /// WoT [type][Type] to be used.
    fn type_() -> Type {
        Type::Object
    }

    /// Deviations from the default [property description][PropertyDescription].
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description
    }

    /// Serialize the value.
    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError>;

    /// Deserialize the value.
    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError>;
}

/// A simplification of [Value] which requires [Serialize] and [DeserializeOwned] to auto-implement [Value].
///
/// # Examples
/// ```
/// # use serde::{Serialize, Deserialize};
/// # use gateway_addon_rust::property::SimpleValue;
/// #[derive(Serialize, Deserialize, Clone, Default)]
/// struct Foo {
///     bar: i32,
/// }
/// impl SimpleValue for Foo {}
/// ```
pub trait SimpleValue:
    Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static
{
    /// WoT [type][Type] to be used.
    fn type_() -> Type {
        Type::Object
    }

    /// Deviations from the default [property description][PropertyDescription].
    fn description(description: PropertyDescription<Self>) -> PropertyDescription<Self> {
        description
    }

    /// Serialize the value.
    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
        Ok(Some(
            serde_json::to_value(value).map_err(ApiError::Serialization)?,
        ))
    }

    /// Deserialize the value.
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

impl SimpleValue for serde_json::Value {
    fn deserialize(value: Option<serde_json::Value>) -> Result<Self, ApiError> {
        match value {
            Some(value) => serde_json::from_value(value).map_err(ApiError::Serialization),
            None => Ok(json!(null)),
        }
    }
}

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
            .map(|e| e.into_iter().map(Some).collect());
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

#[cfg(test)]
mod tests {
    use crate::property::{self, Value};
    use serde_json::json;

    #[test]
    fn test_serialize_bool() {
        assert_eq!(bool::serialize(true).unwrap(), Some(json!(true)));
        assert_eq!(bool::serialize(false).unwrap(), Some(json!(false)));
    }

    #[test]
    fn test_deserialize_bool() {
        assert!(bool::deserialize(Some(json!(true))).unwrap());
        assert!(!bool::deserialize(Some(json!(false))).unwrap());
        assert!(bool::deserialize(None).is_err());
        assert!(bool::deserialize(Some(json!(null))).is_err());
        assert!(bool::deserialize(Some(json!(21))).is_err());
    }

    #[test]
    fn test_serialize_u8() {
        assert_eq!(u8::serialize(142).unwrap(), Some(json!(142)));
    }

    #[test]
    fn test_deserialize_u8() {
        assert_eq!(u8::deserialize(Some(json!(42))).unwrap(), 42);
        assert!(u8::deserialize(None).is_err());
        assert!(u8::deserialize(Some(json!(null))).is_err());
        assert!(u8::deserialize(Some(json!(312))).is_err());
    }

    #[test]
    fn test_serialize_i32() {
        assert_eq!(i32::serialize(5).unwrap(), Some(json!(5)));
        assert_eq!(i32::serialize(-12).unwrap(), Some(json!(-12)));
    }

    #[test]
    fn test_deserialize_i32() {
        assert_eq!(i32::deserialize(Some(json!(42))).unwrap(), 42);
        assert!(i32::deserialize(None).is_err());
        assert!(i32::deserialize(Some(json!(null))).is_err());
        assert!(i32::deserialize(Some(json!(3.5_f32))).is_err());
    }

    #[test]
    fn test_serialize_f32() {
        assert_eq!(f32::serialize(13.5_f32).unwrap(), Some(json!(13.5_f32)));
        assert_eq!(f32::serialize(-11_f32).unwrap(), Some(json!(-11_f32)));
    }

    #[test]
    fn test_deserialize_f32() {
        assert_eq!(f32::deserialize(Some(json!(4.2))).unwrap(), 4.2);
        assert!(f32::deserialize(None).is_err());
        assert!(f32::deserialize(Some(json!(null))).is_err());
        assert!(f32::deserialize(Some(json!("foo"))).is_err());
    }

    #[test]
    fn test_serialize_opti32() {
        assert_eq!(Option::<i32>::serialize(Some(42)).unwrap(), Some(json!(42)));
        assert_eq!(Option::<i32>::serialize(None).unwrap(), None);
    }

    #[test]
    fn test_deserialize_opti32() {
        assert_eq!(
            Option::<i32>::deserialize(Some(json!(42))).unwrap(),
            Some(42)
        );
        assert_eq!(Option::<i32>::deserialize(Some(json!(null))).unwrap(), None);
        assert_eq!(Option::<i32>::deserialize(None).unwrap(), None);
        assert!(Option::<i32>::deserialize(Some(json!("foo"))).is_err());
    }

    #[test]
    fn test_serialize_veci32() {
        assert_eq!(Vec::<i32>::serialize(vec![]).unwrap(), Some(json!([])));
        assert_eq!(
            Vec::<i32>::serialize(vec![21, 42]).unwrap(),
            Some(json!([21, 42]))
        );
    }

    #[test]
    fn test_deserialize_veci32() {
        assert_eq!(
            Vec::<i32>::deserialize(Some(json!([]))).unwrap(),
            Vec::<i32>::new()
        );
        assert_eq!(
            Vec::<i32>::deserialize(Some(json!([21, 42]))).unwrap(),
            vec![21, 42]
        );
        assert!(Vec::<i32>::deserialize(Some(json!(null))).is_err());
        assert!(Vec::<i32>::deserialize(None).is_err());
        assert!(Vec::<i32>::deserialize(Some(json!(42))).is_err());
    }

    #[test]
    fn test_serialize_string() {
        assert_eq!(String::serialize("".to_owned()).unwrap(), Some(json!("")));
        assert_eq!(
            String::serialize("foo".to_owned()).unwrap(),
            Some(json!("foo"))
        );
    }

    #[test]
    fn test_deserialize_string() {
        assert_eq!(String::deserialize(Some(json!(""))).unwrap(), "".to_owned());
        assert_eq!(
            String::deserialize(Some(json!("foo"))).unwrap(),
            "foo".to_owned()
        );
        assert!(String::deserialize(None).is_err());
        assert!(String::deserialize(Some(json!(null))).is_err());
        assert!(String::deserialize(Some(json!(42))).is_err());
    }

    #[test]
    fn test_serialize_jsonvalue() {
        assert_eq!(
            serde_json::Value::serialize(json!(true)).unwrap(),
            Some(json!(true))
        );
        assert_eq!(
            serde_json::Value::serialize(json!(32)).unwrap(),
            Some(json!(32))
        );
        assert_eq!(
            serde_json::Value::serialize(json!("foo".to_owned())).unwrap(),
            Some(json!("foo"))
        );
        assert_eq!(
            serde_json::Value::serialize(json!(null)).unwrap(),
            Some(json!(null))
        );
    }

    #[test]
    fn test_deserialize_jsonvalue() {
        assert_eq!(
            serde_json::Value::deserialize(Some(json!(true))).unwrap(),
            json!(true).to_owned()
        );
        assert_eq!(
            serde_json::Value::deserialize(Some(json!(42))).unwrap(),
            json!(42)
        );
        assert_eq!(
            serde_json::Value::deserialize(Some(json!("foo"))).unwrap(),
            json!("foo")
        );
        assert_eq!(serde_json::Value::deserialize(None).unwrap(), json!(null));
        assert_eq!(
            serde_json::Value::deserialize(Some(json!(null))).unwrap(),
            json!(null)
        );
    }

    #[derive(Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestValueObject {
        b: bool,
    }

    #[derive(Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestValue {
        i: i32,
        s: String,
        o: TestValueObject,
    }

    impl property::SimpleValue for TestValue {}

    #[test]
    fn test_deserialize_testvalue() {
        assert_eq!(
            TestValue::deserialize(Some(json!({"i": 42, "s": "foo", "o": {"b": true}}))).unwrap(),
            TestValue {
                i: 42,
                s: "foo".to_owned(),
                o: TestValueObject { b: true }
            }
        );
        assert!(
            TestValue::deserialize(Some(json!({"i": 42, "s": "foo", "o": {"b": 42}}))).is_err()
        );
        assert!(TestValue::deserialize(Some(json!(42))).is_err());
        assert!(TestValue::deserialize(Some(json!(null))).is_err());
        assert!(TestValue::deserialize(None).is_err());
    }

    #[test]
    fn test_serialize_testvalue() {
        assert_eq!(
            TestValue::serialize(TestValue {
                i: 42,
                s: "foo".to_owned(),
                o: TestValueObject { b: true }
            })
            .unwrap(),
            Some(json!({"i": 42, "s": "foo", "o": {"b": true}}))
        );
    }
}
