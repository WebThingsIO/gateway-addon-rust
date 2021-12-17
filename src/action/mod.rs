/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT actions.

mod action_description;
mod action_handle;
mod action_input;
mod action_trait;

pub use action_description::*;
pub use action_handle::*;
pub use action_input::*;
pub use action_trait::*;

/// Convenience type for a collection of [ActionBase].
pub type Actions = Vec<Box<dyn ActionBase>>;

/// Convenience macro for building an [Actions].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, example::ExampleAction};
/// actions![ExampleAction::new()]
/// # ;
/// ```
#[macro_export]
macro_rules! actions [
    ($($e:expr),*) => ({
        let _temp: Actions = vec![$(Box::new($e)),*];
        _temp
    })
];

#[cfg(test)]
pub(crate) mod tests {
    pub use super::action_trait::tests::*;
}
