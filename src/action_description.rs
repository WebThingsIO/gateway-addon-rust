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

/// A struct which represents a WoT [action description][webthings_gateway_ipc_types::Action].
///
/// This is used by [Action][crate::action::Action].
///
/// Use the provided builder methods instead of directly writing to the struct fields.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, action::AtType};
/// # let _ =
/// ActionDescription::<i32>::default()
///     .at_type(AtType::FadeAction)
///     .title("Foo fade action")
///     .description("Fade your foo to bar")
/// # ;
/// ```
#[derive(Clone)]
pub struct ActionDescription<T: Input> {
    pub at_type: Option<AtType>,
    pub description: Option<String>,
    pub input: Option<serde_json::Value>,
    pub links: Option<Vec<Link>>,
    pub title: Option<String>,
    pub _input: PhantomData<T>,
}

/// A trait which converts WoT [types][crate::type_::Type] to Rust types.
///
/// Already implemented for common Rust types. You may want to implement [SimpleInput] instead of this.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*,  action::{AtType, Input}};
/// # use serde_json::json;
/// # use serde::{de::Error, Deserialize};
/// #[derive(Clone)]
/// struct FadeInput{
///     level: u8,
///     duration: u32,
/// }
///
/// impl Input for FadeInput {
///     fn input() -> Option<serde_json::Value>{
///         Some(json!({
///             "type": "object",
///             "properties": {
///                 "level": {
///                     "type": "integer",
///                     "unit": "percent",
///                     "minimum": 0,
///                     "maximum": 100,
///                 },
///                 "duration": {
///                     "type": "integer",
///                     "unit": "second",
///                     "minimum": 0,
///                 }
///             }
///         }))
///     }
///
///     fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
///         description.at_type(AtType::FadeAction)
///     }
///
///     fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
///         Ok(Self{
///             level: value.as_object().unwrap().get("level").unwrap().as_u64().unwrap() as _,
///             duration: value.as_object().unwrap().get("duration").unwrap().as_u64().unwrap() as _,
///         })
///     }
/// }
/// ```
pub trait Input: Clone + Send + Sync + 'static {
    /// WoT type to be used in the form of a json schema.
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
        }))
    }

    /// Deviations from the default [action description][ActionDescription].
    fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
        description
    }

    /// Deserialize the value.
    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError>;
}

/// A simplification of [Input] which requires [DeserializeOwned] and [JsonSchema] to auto-implement [Input].
///
/// # Examples
/// ```
/// # use serde::Deserialize;
/// # use schemars::JsonSchema;
/// # use gateway_addon_rust::action::SimpleInput;
/// #[derive(Deserialize, JsonSchema, Clone)]
/// struct Foo {
///     bar: i32,
/// }
/// impl SimpleInput for Foo {}
/// ```
pub trait SimpleInput: DeserializeOwned + JsonSchema + Clone + Send + Sync + 'static {
    /// WoT type to be used in the form of a json schema.
    fn input() -> Option<serde_json::Value> {
        if let Ok(schema) = serde_json::to_value(&schema_for!(Self)) {
            Some(schema)
        } else {
            None
        }
    }

    /// Deviations from the default [action description][ActionDescription].
    fn description(description: ActionDescription<Self>) -> ActionDescription<Self> {
        description
    }

    /// Deserialize the value.
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

/// A struct which can be used as [input][Input] for actions which do not expect any input.
#[derive(Clone, PartialEq, Debug)]
pub struct NoInput;

impl Input for NoInput {
    fn input() -> Option<serde_json::Value> {
        None
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        if value == serde_json::Value::Null {
            Ok(NoInput)
        } else {
            Err(ApiError::Serialization(serde_json::Error::custom(
                "Expected no input",
            )))
        }
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
        T::input().map(|mut input| {
            if let serde_json::Value::Object(ref mut map) = input {
                if let Some(type_) = map.get_mut("type") {
                    if let serde_json::Value::Array(ref mut array) = type_ {
                        array.push(json!("null"));
                    } else {
                        *type_ = json!([type_, "null"]);
                    }
                    *type_ = json!(type_)
                }
            }
            input
        })
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, ApiError> {
        Ok(match value {
            serde_json::Value::Null => None,
            _ => Some(T::deserialize(value)?),
        })
    }
}

/// Possible values of `@type` for an [action][ActionDescription].
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

/// # Builder methods
impl<T: Input> ActionDescription<T> {
    /// Build an empty [ActionDescription].
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

    /// Manually overwrite `input`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::{action::ActionDescription};
    /// # use serde_json::json;
    /// ActionDescription::<serde_json::Value>::default().input(json!({
    ///     "type": "number",
    ///     "multipleOf": 2,
    /// }))
    /// # ;
    /// ```
    pub fn input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
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
    /// # use gateway_addon_rust::action::ActionDescription;
    /// # use webthings_gateway_ipc_types::Link;
    /// # let _: ActionDescription<i32> =
    /// ActionDescription::default()
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

    /// Set `title`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
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

#[cfg(test)]
mod tests {
    use crate::action_description::{self, Input, NoInput};
    use schemars::JsonSchema;
    use serde_json::json;

    #[test]
    fn test_deserialize_noinput() {
        assert_eq!(NoInput::deserialize(json!(null)).unwrap(), NoInput);
        assert!(NoInput::deserialize(json!(21)).is_err());
    }

    #[test]
    fn test_deserialize_bool() {
        assert_eq!(bool::deserialize(json!(true)).unwrap(), true);
        assert_eq!(bool::deserialize(json!(false)).unwrap(), false);
        assert!(bool::deserialize(json!(null)).is_err());
        assert!(bool::deserialize(json!(21)).is_err());
    }

    #[test]
    fn test_deserialize_u8() {
        assert_eq!(u8::deserialize(json!(42)).unwrap(), 42);
        assert!(u8::deserialize(json!(null)).is_err());
        assert!(u8::deserialize(json!(312)).is_err());
    }

    #[test]
    fn test_deserialize_i32() {
        assert_eq!(i32::deserialize(json!(42)).unwrap(), 42);
        assert!(i32::deserialize(json!(null)).is_err());
        assert!(i32::deserialize(json!(3.5_f32)).is_err());
    }

    #[test]
    fn test_deserialize_f32() {
        assert_eq!(f32::deserialize(json!(4.2)).unwrap(), 4.2);
        assert!(f32::deserialize(json!(null)).is_err());
        assert!(f32::deserialize(json!("foo")).is_err());
    }

    #[test]
    fn test_deserialize_opti32() {
        assert_eq!(Option::<i32>::deserialize(json!(42)).unwrap(), Some(42));
        assert_eq!(Option::<i32>::deserialize(json!(null)).unwrap(), None);
        assert!(Option::<i32>::deserialize(json!("foo")).is_err());
    }

    #[test]
    fn test_deserialize_veci32() {
        assert_eq!(
            Vec::<i32>::deserialize(json!([])).unwrap(),
            Vec::<i32>::new().to_owned()
        );
        assert_eq!(
            Vec::<i32>::deserialize(json!([21, 42])).unwrap(),
            vec![21, 42]
        );
        assert!(Vec::<i32>::deserialize(json!(null)).is_err());
        assert!(Vec::<i32>::deserialize(json!(42)).is_err());
    }

    #[test]
    fn test_deserialize_string() {
        assert_eq!(String::deserialize(json!("")).unwrap(), "".to_owned());
        assert_eq!(String::deserialize(json!("foo")).unwrap(), "foo".to_owned());
        assert!(String::deserialize(json!(null)).is_err());
        assert!(String::deserialize(json!(42)).is_err());
    }

    #[test]
    fn test_deserialize_jsonvalue() {
        assert_eq!(
            serde_json::Value::deserialize(json!(true)).unwrap(),
            json!(true).to_owned()
        );
        assert_eq!(
            serde_json::Value::deserialize(json!(42)).unwrap(),
            json!(42).to_owned()
        );
        assert_eq!(
            serde_json::Value::deserialize(json!("foo")).unwrap(),
            json!("foo").to_owned()
        );
        assert_eq!(
            serde_json::Value::deserialize(json!(null)).unwrap(),
            json!(null)
        );
    }

    #[derive(Clone, JsonSchema, serde::Deserialize, PartialEq, Debug)]
    struct TestInputObject {
        b: bool,
    }

    #[derive(Clone, JsonSchema, serde::Deserialize, PartialEq, Debug)]
    struct TestInput {
        i: i32,
        s: String,
        o: TestInputObject,
    }

    impl action_description::SimpleInput for TestInput {}

    #[test]
    fn test_deserialize_testinput() {
        assert_eq!(
            TestInput::deserialize(json!({"i": 42, "s": "foo", "o": {"b": true}})).unwrap(),
            TestInput {
                i: 42,
                s: "foo".to_owned(),
                o: TestInputObject { b: true }
            }
        );
        assert!(TestInput::deserialize(json!({"i": 42, "s": "foo", "o": {"b": 42}})).is_err());
        assert!(TestInput::deserialize(json!(42)).is_err());
        assert!(TestInput::deserialize(json!(null)).is_err());
    }
}
