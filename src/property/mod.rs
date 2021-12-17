/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT properties.

mod property_builder;
mod property_description;
mod property_handle;
mod property_trait;
mod property_value;

pub use property_builder::*;
pub use property_description::*;
pub use property_handle::*;
pub use property_trait::*;
pub use property_value::*;

/// Convenience type for a collection of [PropertyBuilderBase].
pub type Properties = Vec<Box<dyn PropertyBuilderBase>>;

/// Convenience macro for building a [Properties].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, example::ExamplePropertyBuilder};
/// properties![ExamplePropertyBuilder::new()]
/// # ;
/// ```
#[macro_export]
macro_rules! properties [
    ($($e:expr),*) => ({
        let _temp: Properties = vec![$(Box::new($e)),*];
        _temp
    })
];

#[cfg(test)]
pub(crate) mod tests {
    pub use super::{property_builder::tests::*, property_trait::tests::*};
}
