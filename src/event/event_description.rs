/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{error::WebthingsError, event::Data, type_::Type};
use std::marker::PhantomData;
use webthings_gateway_ipc_types::{Event as FullEventDescription, Link};

/// A struct which represents a WoT [event description][webthings_gateway_ipc_types::Event].
///
/// This is used by [Event][crate::Event].
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
    pub fn into_full_description(
        self,
        name: String,
    ) -> Result<FullEventDescription, WebthingsError> {
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
