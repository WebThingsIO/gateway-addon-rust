/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{error::WebthingsError, property::Value, type_::Type};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Link, Property as FullPropertyDescription};

/// A struct which represents a WoT [property description][webthings_gateway_ipc_types::Property].
///
/// This is used by [PropertyBuilder][crate::property::PropertyBuilder].
///
/// Use the provided builder methods instead of directly writing to the struct fields.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::AtType};
/// # let _ =
/// PropertyDescription::<i32>::default()
///     .at_type(AtType::LevelProperty)
///     .title("Foo concentration")
///     .unit("bar")
///     .maximum(1000)
///     .multiple_of(5)
/// # ;
/// ```
#[derive(Clone)]
pub struct PropertyDescription<T: Value> {
    pub at_type: Option<AtType>,
    pub description: Option<String>,
    pub enum_: Option<Vec<T>>,
    pub links: Option<Vec<Link>>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub read_only: Option<bool>,
    pub title: Option<String>,
    pub type_: Type,
    pub unit: Option<String>,
    pub value: T,
    pub visible: Option<bool>,
    _value: PhantomData<T>,
}

/// Possible values of `@type` for a [property][PropertyDescription].
#[derive(Debug, Clone)]
pub enum AtType {
    AlarmProperty,
    BarometricPressureProperty,
    BooleanProperty,
    BrightnessProperty,
    ColorModeProperty,
    ColorProperty,
    ColorTemperatureProperty,
    ConcentrationProperty,
    CurrentProperty,
    DensityProperty,
    FrequencyProperty,
    HeatingCoolingProperty,
    HumidityProperty,
    ImageProperty,
    InstantaneousPowerFactorProperty,
    InstantaneousPowerProperty,
    LeakProperty,
    LevelProperty,
    LockedProperty,
    MotionProperty,
    OnOffProperty,
    OpenProperty,
    PushedProperty,
    SmokeProperty,
    TargetTemperatureProperty,
    TemperatureProperty,
    ThermostatModeProperty,
    VideoProperty,
    VoltageProperty,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// # Builder methods
impl<T: Value> PropertyDescription<T> {
    /// Build an empty [PropertyDescription].
    pub fn default() -> Self {
        T::description(Self {
            at_type: None,
            description: None,
            enum_: None,
            links: None,
            maximum: None,
            minimum: None,
            multiple_of: None,
            read_only: None,
            title: None,
            type_: T::type_(),
            unit: None,
            value: T::default(),
            visible: None,
            _value: PhantomData,
        })
    }

    /// Set `@type`.
    #[must_use]
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type);
        self
    }

    /// Set `description`.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set `enum`.
    #[must_use]
    pub fn enum_(mut self, enum_: Vec<T>) -> Self {
        self.enum_ = Some(enum_);
        self
    }

    /// Set `links`.
    #[must_use]
    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    /// Add a single link to `links`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::property::PropertyDescription;
    /// # use webthings_gateway_ipc_types::Link;
    /// # let _: PropertyDescription<i32> =
    /// PropertyDescription::default()
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
    #[must_use]
    pub fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    /// Set `maximum`.
    #[must_use]
    pub fn maximum<F: Into<f64>>(mut self, maximum: F) -> Self {
        self.maximum = Some(maximum.into());
        self
    }

    /// Set `minimum`.
    #[must_use]
    pub fn minimum<F: Into<f64>>(mut self, minimum: F) -> Self {
        self.minimum = Some(minimum.into());
        self
    }

    /// Set `multipleOf`.
    #[must_use]
    pub fn multiple_of<F: Into<f64>>(mut self, multiple_of: F) -> Self {
        self.multiple_of = Some(multiple_of.into());
        self
    }

    /// Set `readOnly`.
    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }

    /// Set `title`.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Manually overwrite `type`.
    ///
    /// # Examples
    /// ```
    /// # use gateway_addon_rust::{type_::Type, property::PropertyDescription};
    /// PropertyDescription::<serde_json::Value>::default().type_(Type::Number)
    /// # ;
    /// ```
    #[must_use]
    pub fn type_(mut self, type_: Type) -> Self {
        self.type_ = type_;
        self
    }

    /// Set `unit`.
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set initial `value`.
    #[must_use]
    pub fn value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    /// Set `visible`.
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    #[doc(hidden)]
    pub fn into_full_description(
        self,
        name: String,
    ) -> Result<FullPropertyDescription, WebthingsError> {
        let enum_ = if let Some(enum_) = self.enum_ {
            let mut v = Vec::new();
            for e in enum_ {
                v.push(T::serialize(e)?.ok_or_else(|| {
                    WebthingsError::Serialization(<serde_json::Error as serde::ser::Error>::custom(
                        "Expected Some, found None",
                    ))
                })?);
            }
            Some(v)
        } else {
            None
        };
        Ok(FullPropertyDescription {
            at_type: self.at_type.map(|t| t.to_string()),
            description: self.description,
            enum_,
            links: self.links,
            maximum: self.maximum,
            minimum: self.minimum,
            multiple_of: self.multiple_of,
            read_only: self.read_only,
            title: self.title,
            type_: self.type_.to_string(),
            unit: self.unit,
            value: T::serialize(self.value)?,
            visible: self.visible,
            name: Some(name),
        })
    }
}
