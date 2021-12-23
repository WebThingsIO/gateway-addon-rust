/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO API Handlers.

pub(crate) mod api_handler_message_handler;
mod api_handler_trait;

pub use api_handler_trait::*;

#[cfg(test)]
pub(crate) mod tests {
    pub use super::api_handler_trait::tests::*;
}
