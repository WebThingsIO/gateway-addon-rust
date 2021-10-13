/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use schemars::{schema_for, JsonSchema};
use serde_json::Value;
use std::marker::PhantomData;
use webthings_gateway_ipc_types::Link;

pub struct ActionDescription<I: JsonSchema> {
    pub at_type: Option<String>,
    pub description: Option<String>,
    pub input: Option<Value>,
    pub links: Option<Vec<Link>>,
    pub title: Option<String>,
    pub input_: PhantomData<I>,
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

pub trait ActionDescriptionBuilder {
    fn at_type(self, at_type: AtType) -> Self;
    fn description<S: Into<String>>(self, description: S) -> Self;
    fn input(self, input: Value) -> Self;
    fn links(self, links: Vec<Link>) -> Self;
    fn link(self, links: Link) -> Self;
    fn title<S: Into<String>>(self, title: S) -> Self;
    fn default() -> Self;
}

impl<I: JsonSchema> ActionDescriptionBuilder for ActionDescription<I> {
    fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    fn input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    fn default() -> Self {
        let input = schema_for!(I);
        let input = if &I::schema_name() == "no input" {
            None
        } else {
            serde_json::to_value(input).ok()
        };
        Self {
            at_type: None,
            description: None,
            links: None,
            title: None,
            input,
            input_: PhantomData,
        }
    }
}
