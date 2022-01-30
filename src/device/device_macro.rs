/// Use this on a struct to generate a built device around it, including useful impls.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, device::DeviceBuilder};
/// # use async_trait::async_trait;
/// #[device]
/// struct ExampleDevice {
///     foo: i32,
/// }
///
/// impl DeviceStructure for ExampleDevice {
///     // ...
/// #   fn id(&self) -> String {
/// #       "example-device".to_owned()
/// #   }
/// #   fn description(&self) -> DeviceDescription {
/// #       DeviceDescription::default()
/// #   }
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {
///     // ...
/// }
/// ```
/// will expand to
/// ```
/// # use gateway_addon_rust::{prelude::*, device::{BuiltDevice, DeviceBuilder}};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleDevice {
///     foo: i32,
/// }
///
/// struct BuiltExampleDevice {
///     data: ExampleDevice,
///     device_handle: DeviceHandle,
/// }
///
/// impl BuiltDevice for BuiltExampleDevice {
///     fn device_handle(&self) -> &DeviceHandle {
///         &self.device_handle
///     }
///     fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///         &mut self.device_handle
///     }
/// }
///
/// impl DeviceBuilder for ExampleDevice {
///     type BuiltDevice = BuiltExampleDevice;
///     fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
///         BuiltExampleDevice {
///             data,
///             device_handle,
///         }
///     }
/// }
///
/// impl Deref for BuiltExampleDevice {
///     type Target = ExampleDevice;
///     fn deref(&self) -> &Self::Target {
///         &self.data
///     }
/// }
///
/// impl DerefMut for BuiltExampleDevice {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.data
///     }
/// }
///
/// impl DeviceStructure for ExampleDevice {
///     // ...
/// #   fn id(&self) -> String {
/// #       "example-device".to_owned()
/// #   }
/// #   fn description(&self) -> DeviceDescription {
/// #       DeviceDescription::default()
/// #   }
/// }
///
/// #[async_trait]
/// impl Device for BuiltExampleDevice {
///     // ...
/// }
/// ```
pub use gateway_addon_rust_codegen::device;
