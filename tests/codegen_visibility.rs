/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use gateway_addon_rust::prelude::*;

mod private_module {
    use gateway_addon_rust::prelude::*;

    #[adapter]
    pub struct TestAdapter;
}

impl Adapter for private_module::BuiltTestAdapter {}
