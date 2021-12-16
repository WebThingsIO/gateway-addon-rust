/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{action::Input};



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
