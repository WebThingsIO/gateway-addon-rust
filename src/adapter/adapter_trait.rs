/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::AdapterHandle;
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::time::Duration;
use webthings_gateway_ipc_types::DeviceWithoutId;

/// A trait used to specify the behaviour of a WebthingsIO adapter.
///
/// Defines how to react on gateway requests.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleDevice, error::WebthingsError, adapter::BuiltAdapter};
/// # use webthings_gateway_ipc_types::DeviceWithoutId;
/// # use async_trait::async_trait;
/// # use as_any::Downcast;
/// #[adapter]
/// struct ExampleAdapter { foo: i32 }
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
/// #[async_trait]
/// impl Adapter for BuiltExampleAdapter {
///     async fn on_unload(&mut self) -> Result<(), String> {
///         println!("Foo: {}", self.foo);
///         Ok(())
///     }
/// }
///
/// # impl ExampleAdapter {
/// #     pub fn new(foo: i32) -> Self {
/// #         Self { foo }
/// #     }
/// # }
/// impl BuiltExampleAdapter {
///     pub async fn init(&mut self) -> Result<(), WebthingsError> {
///         self.adapter_handle_mut()
///             .add_device(ExampleDevice::new())
///             .await?;
///         Ok(())
///     }
/// }
///
/// # #[tokio::main]
/// pub async fn main() -> Result<(), WebthingsError> {
///     let mut plugin = connect("example-addon").await?;
///     let adapter = plugin
///         .add_adapter(ExampleAdapter::new(42))
///         .await?;
///     adapter
///         .lock()
///         .await
///         .downcast_mut::<BuiltExampleAdapter>()
///         .unwrap()
///         .init()
///         .await?;
///     plugin.event_loop().await;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait Adapter: BuiltAdapter + Send + Sync + AsAny + 'static {
    /// Called when this Adapter should be unloaded.
    async fn on_unload(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when a new [device][crate::Device] was saved within the gateway.
    ///
    /// This happens when a thing was added through the add things view.
    async fn on_device_saved(
        &mut self,
        _device_id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Called when the gateway starts pairing.
    ///
    /// This happens when the add things view opens.
    async fn on_start_pairing(&mut self, _timeout: Duration) -> Result<(), String> {
        Ok(())
    }

    /// Called when the gateway stops pairing.
    ///
    /// This happens when the add things view closes.
    async fn on_cancel_pairing(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when a previously saved [device][crate::Device] was removed.
    ///
    /// This happens when an added thing was removed through the gateway.
    async fn on_remove_device(&mut self, _device_id: String) -> Result<(), String> {
        Ok(())
    }
}

impl Downcast for dyn Adapter {}

/// A trait used to wrap an [adapter handle][AdapterHandle].
///
/// When you use the [adapter][macro@crate::adapter] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, adapter::BuiltAdapter};
/// # use async_trait::async_trait;
/// struct BuiltExampleAdapter {
///     adapter_handle: AdapterHandle,
/// }
///
/// impl BuiltAdapter for BuiltExampleAdapter {
///     fn adapter_handle(&self) -> &AdapterHandle {
///         &self.adapter_handle
///     }
///     fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
///         &mut self.adapter_handle
///     }
/// }
/// ```
pub trait BuiltAdapter {
    /// Return a reference to the wrapped [adapter handle][AdapterHandle].
    fn adapter_handle(&self) -> &AdapterHandle;

    /// Return a mutable reference to the wrapped [adapter handle][AdapterHandle].
    fn adapter_handle_mut(&mut self) -> &mut AdapterHandle;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        adapter::{tests::MockAdapter, BuiltAdapter},
        Adapter, AdapterHandle,
    };
    use async_trait::async_trait;
    use std::time::Duration;
    use webthings_gateway_ipc_types::DeviceWithoutId;

    pub struct BuiltMockAdapter {
        data: MockAdapter,
        adapter_handle: AdapterHandle,
    }

    impl BuiltMockAdapter {
        pub fn new(data: MockAdapter, adapter_handle: AdapterHandle) -> Self {
            Self {
                data,
                adapter_handle,
            }
        }
    }

    impl BuiltAdapter for BuiltMockAdapter {
        fn adapter_handle(&self) -> &AdapterHandle {
            &self.adapter_handle
        }

        fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
            &mut self.adapter_handle
        }
    }

    impl std::ops::Deref for BuiltMockAdapter {
        type Target = MockAdapter;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl std::ops::DerefMut for BuiltMockAdapter {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    #[async_trait]
    impl Adapter for BuiltMockAdapter {
        async fn on_unload(&mut self) -> Result<(), String> {
            self.adapter_helper.on_unload().await
        }

        async fn on_start_pairing(&mut self, timeout: Duration) -> Result<(), String> {
            self.adapter_helper.on_start_pairing(timeout).await
        }

        async fn on_cancel_pairing(&mut self) -> Result<(), String> {
            self.adapter_helper.on_cancel_pairing().await
        }

        async fn on_device_saved(
            &mut self,
            device_id: String,
            device_description: DeviceWithoutId,
        ) -> Result<(), String> {
            self.adapter_helper
                .on_device_saved(device_id, device_description)
                .await
        }

        async fn on_remove_device(&mut self, device_id: String) -> Result<(), String> {
            self.adapter_helper.on_remove_device(device_id).await
        }
    }
}
