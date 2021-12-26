/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! Connection to the WebthingsIO gateway.

mod plugin_connection;
pub(crate) mod plugin_message_handler;
mod plugin_struct;

pub use plugin_connection::*;
pub use plugin_struct::*;

#[cfg(test)]
pub(crate) mod tests {
    pub use super::plugin_struct::tests::*;
}
