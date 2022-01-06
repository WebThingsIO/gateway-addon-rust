/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{Adapter, AdapterHandle};
use as_any::AsAny;

/// A trait used to specify the structure of a WebthingsIO adapter.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleDevice, error::WebthingsError, adapter::BuiltAdapter};
/// # use webthings_gateway_ipc_types::DeviceWithoutId;
/// # use async_trait::async_trait;
/// # use as_any::Downcast;
/// struct ExampleAdapter { foo: i32 }
///
/// impl AdapterStructure for ExampleAdapter {
///     fn id(&self) -> String {
///         "example-adapter".to_owned()
///     }
///
///     fn name(&self) -> String {
///         "Example Adapter".to_owned()
///     }
/// }
/// ```
pub trait AdapterStructure: Send + Sync + AsAny + 'static {
    /// ID of the adapter.
    fn id(&self) -> String;

    /// Name of the adapter.
    fn name(&self) -> String;
}

/// A trait used to build an [Adapter] around a data struct and an [adapter handle][AdapterHandle].
///
/// When you use the [adapter][macro@crate::adapter] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, adapter::{BuiltAdapter, AdapterBuilder}};
/// # use async_trait::async_trait;
/// struct ExampleAdapter {
///     foo: i32,
/// }
///
/// impl AdapterStructure for ExampleAdapter {
///     // ...
///     # fn id(&self) -> String {
///     #     "example-adapter".to_owned()
///     # }
///     # fn name(&self) -> String {
///     #     "Example Adapter".to_owned()
///     # }
/// }
///
/// struct BuiltExampleAdapter {
///     data: ExampleAdapter,
///     adapter_handle: AdapterHandle,
/// }
///
/// impl BuiltAdapter for BuiltExampleAdapter {
///     // ...
/// #   fn adapter_handle(&self) -> &AdapterHandle {
/// #       &self.adapter_handle
/// #   }
/// #   fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
/// #       &mut self.adapter_handle
/// #   }
/// }
///
/// #[async_trait]
/// impl Adapter for BuiltExampleAdapter {}
///
/// impl AdapterBuilder for ExampleAdapter {
///     type BuiltAdapter = BuiltExampleAdapter;
///     fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter {
///         BuiltExampleAdapter {
///             data,
///             adapter_handle,
///         }
///     }
/// }
/// ```
pub trait AdapterBuilder: AdapterStructure {
    /// Type of [Adapter] to build.
    type BuiltAdapter: Adapter;

    /// Build the [adapter][Adapter] from a data struct and an [adapter handle][AdapterHandle].
    fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        adapter::{tests::BuiltMockAdapter, AdapterBuilder},
        AdapterHandle, AdapterStructure,
    };
    use mockall::mock;
    use std::time::Duration;
    use webthings_gateway_ipc_types::DeviceWithoutId;

    mock! {
        pub AdapterHelper {
            pub async fn on_unload(&mut self) -> Result<(), String>;
            pub async fn on_start_pairing(&mut self, timeout: Duration) -> Result<(), String>;
            pub async fn on_cancel_pairing(&mut self) -> Result<(), String>;
            pub async fn on_device_saved(
                &mut self,
                device_id: String,
                device_description: DeviceWithoutId
            ) -> Result<(), String>;
            pub async fn on_remove_device(&mut self, device_id: String) -> Result<(), String>;
        }
    }

    pub struct MockAdapter {
        adapter_name: String,
        pub adapter_helper: MockAdapterHelper,
    }

    impl MockAdapter {
        pub fn new(adapter_name: String) -> Self {
            Self {
                adapter_name,
                adapter_helper: MockAdapterHelper::new(),
            }
        }
    }

    impl std::ops::Deref for MockAdapter {
        type Target = MockAdapterHelper;
        fn deref(&self) -> &Self::Target {
            &self.adapter_helper
        }
    }

    impl std::ops::DerefMut for MockAdapter {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.adapter_helper
        }
    }

    impl AdapterStructure for MockAdapter {
        fn id(&self) -> String {
            self.adapter_name.to_owned()
        }

        fn name(&self) -> String {
            self.adapter_name.to_owned()
        }
    }

    impl AdapterBuilder for MockAdapter {
        type BuiltAdapter = BuiltMockAdapter;
        fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter {
            BuiltMockAdapter::new(data, adapter_handle)
        }
    }
}
