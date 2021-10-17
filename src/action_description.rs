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

impl<I: JsonSchema> ActionDescription<I> {
    pub fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    pub fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn default() -> Self {
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
