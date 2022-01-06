/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO API Handlers.

mod api_handler_handle;
mod api_handler_macro;
pub(crate) mod api_handler_message_handler;
mod api_handler_trait;

pub use api_handler_handle::*;
pub use api_handler_macro::*;
pub use api_handler_trait::*;

/// An [ApiHandler](crate::api_handler::ApiHandler) request.
pub use webthings_gateway_ipc_types::Request as ApiRequest;
/// An [ApiHandler](crate::api_handler::ApiHandler) response.
pub use webthings_gateway_ipc_types::Response as ApiResponse;

#[cfg(test)]
pub(crate) mod tests {
    pub use super::api_handler_trait::tests::*;
}
