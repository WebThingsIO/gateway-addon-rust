/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! The set of possible errors when working with this crate.

use thiserror::Error;

/// The set of possible errors when working with this crate.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Failed to connect to gateway
    #[error("Failed to connect to gateway")]
    Connect(#[source] tungstenite::Error),

    /// Failed to send message
    #[error("Failed to send message")]
    Send(#[source] tungstenite::Error),

    /// Failed to serialize message
    #[error("Failed to serialize message")]
    Serialization(#[source] serde_json::Error),

    /// Failed to access database
    #[error("Failed to access database")]
    Database(#[source] sqlite::Error),

    /// Unknown property
    #[error("Unknown property")]
    UnknownProperty(String),

    /// Unknown event
    #[error("Unknown event")]
    UnknownEvent(String),

    /// Unknown device
    #[error("Unknown device")]
    UnknownDevice(String),

    /// Unknown adapter
    #[error("Unknown adapter")]
    UnknownAdapter(String),
}
