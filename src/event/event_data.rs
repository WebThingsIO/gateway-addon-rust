/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{api_error::ApiError, type_::Type, EventDescription};
use serde::{ser::Error, Serialize};



/// A trait which converts Rust types to WoT [types][crate::type_::Type].
///
/// Already implemented for common Rust types. You may want to implement [SimpleData] instead of this.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, type_::Type, event::{AtType, Data}};
/// # use serde_json::json;
/// # use serde::{de::Error, Deserialize};
/// #[derive(Clone)]
/// struct OverheatedData(i32);
///
/// impl Data for OverheatedData {
///     fn type_() -> Option<Type> {
///         Some(Type::Number)
///     }
///     fn description(description: EventDescription<Self>) -> EventDescription<Self> {
///         description.at_type(AtType::OverheatedEvent)
///     }
///     fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError> {
///         Ok(Some(
///             serde_json::to_value(value.0).map_err(ApiError::Serialization)?,
///         ))
///     }
/// }
/// ```
pub trait Data: Clone + Send + Sync + 'static {
    /// WoT [type][crate::type_::Type] to be used.
    fn type_() -> Option<Type> {
        Some(Type::Object)
    }

    /// Deviations from the default [event description][EventDescription].
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description
    }

    /// Serialize the value.
    fn serialize(value: Self) -> Result<Option<serde_json::Value>, ApiError>;
}

/// A simplification of [Data] which requires [Serialize] to auto-implement [Data].
///
/// # Examples
/// ```
/// # use serde::Serialize;
/// # use gateway_addon_rust::event::SimpleData;
/// #[derive(Serialize, Clone)]
/// struct Foo {
///     bar: i32,
/// }
/// impl SimpleData for Foo {}
/// ```
pub trait SimpleData: Serialize + Clone + Send + Sync + 'static {
    /// WoT [type][crate::type_::Type] to be used.
    fn type_() -> Option<Type> {
        Some(Type::Object)
    }

    /// Deviations from the default [event description][EventDescription].
    fn description(description: EventDescription<Self>) -> EventDescription<Self> {
        description
    }

    /// Serialize the value.
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

/// A struct which can be used as [data][Data] for events which do not expect any data.
#[derive(Clone, Serialize, PartialEq, Debug)]
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

#[cfg(test)]
mod tests {
    use crate::event::{self, Data, NoData};
    use serde_json::json;

    #[test]
    fn test_serialize_nodata() {
        assert_eq!(NoData::serialize(NoData).unwrap(), None);
    }

    #[test]
    fn test_serialize_bool() {
        assert_eq!(bool::serialize(true).unwrap(), Some(json!(true)));
        assert_eq!(bool::serialize(false).unwrap(), Some(json!(false)));
    }

    #[test]
    fn test_serialize_u8() {
        assert_eq!(u8::serialize(142).unwrap(), Some(json!(142)));
    }

    #[test]
    fn test_serialize_i32() {
        assert_eq!(i32::serialize(5).unwrap(), Some(json!(5)));
        assert_eq!(i32::serialize(-12).unwrap(), Some(json!(-12)));
    }

    #[test]
    fn test_serialize_f32() {
        assert_eq!(f32::serialize(13.5_f32).unwrap(), Some(json!(13.5_f32)));
        assert_eq!(f32::serialize(-11_f32).unwrap(), Some(json!(-11_f32)));
    }

    #[test]
    fn test_serialize_opti32() {
        assert_eq!(Option::<i32>::serialize(Some(42)).unwrap(), Some(json!(42)));
        assert_eq!(Option::<i32>::serialize(None).unwrap(), None);
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
    fn test_serialize_string() {
        assert_eq!(String::serialize("".to_owned()).unwrap(), Some(json!("")));
        assert_eq!(
            String::serialize("foo".to_owned()).unwrap(),
            Some(json!("foo"))
        );
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

    #[derive(Clone, serde::Serialize, PartialEq, Debug)]
    struct TestDataObject {
        b: bool,
    }

    #[derive(Clone, serde::Serialize, PartialEq, Debug)]
    struct TestData {
        i: i32,
        s: String,
        o: TestDataObject,
    }

    impl event::SimpleData for TestData {}

    #[test]
    fn test_serialize_testdata() {
        assert_eq!(
            TestData::serialize(TestData {
                i: 42,
                s: "foo".to_owned(),
                o: TestDataObject { b: true }
            })
            .unwrap(),
            Some(json!({"i": 42, "s": "foo", "o": {"b": true}}))
        );
    }
}
