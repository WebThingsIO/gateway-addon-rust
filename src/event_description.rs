/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{api_error::ApiError, type_::Type};
use serde::{ser::Error, Serialize};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Event as FullEventDescription, Link};

/// A struct which represents a WoT [event description][webthings_gateway_ipc_types::Event].
///
/// This is used by [Event][crate::event::Event].
///
/// Use the provided builder methods instead of directly writing to the struct fields.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::{AtType, NoData}};
/// # let _ =
/// EventDescription::<NoData>::default()
///     .at_type(AtType::OverheatedEvent)
///     .title("Foo overheated event")
///     .description("Your foo is hot")
/// # ;
/// ```
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

/// Possible values of `@type` for an [event][EventDescription].
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

/// # Builder methods
impl<T: Data> EventDescription<T> {
    /// Build an empty [EventDescription].
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

    /// Set `@type`.
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type);
        self
    }

    /// Set `description`.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set `enum`.
    pub fn enum_(mut self, enum_: Vec<T>) -> Self {
        self.enum_ = Some(enum_);
        self
    }

    /// Set `links`.
    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    /// Add a single link to `links`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::event::EventDescription;
    /// # use webthings_gateway_ipc_types::Link;
    /// # let _: EventDescription<i32> =
    /// EventDescription::default()
    ///     .link(Link {
    ///         href: "https://www.rust-lang.org/".to_owned(),
    ///         media_type: None,
    ///         rel: None,
    ///     })
    ///     .link(Link {
    ///         href: "https://www.reddit.com/".to_owned(),
    ///         media_type: None,
    ///         rel: None,
    ///     })
    /// # ;
    /// ```
    pub fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    /// Set `maximum`.
    pub fn maximum<F: Into<f64>>(mut self, maximum: F) -> Self {
        self.maximum = Some(maximum.into());
        self
    }

    /// Set `minimum`.
    pub fn minimum<F: Into<f64>>(mut self, minimum: F) -> Self {
        self.minimum = Some(minimum.into());
        self
    }

    /// Set `multipleOf`.
    pub fn multiple_of<F: Into<f64>>(mut self, multiple_of: F) -> Self {
        self.multiple_of = Some(multiple_of.into());
        self
    }

    /// Set `title`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Manually overwrite `type`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::{type_::Type, event::EventDescription};
    /// EventDescription::<serde_json::Value>::default().type_(Type::Number)
    /// # ;
    /// ```
    pub fn type_(mut self, type_: Type) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Set `unit`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
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

#[cfg(test)]
mod tests {
    use crate::event_description::Data;
    use serde_json::json;

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
}
