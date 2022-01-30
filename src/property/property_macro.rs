/// Use this on a struct to generate a built property around it, including useful impls.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, property::PropertyBuilder};
/// # use async_trait::async_trait;
/// #[property]
/// struct ExampleProperty {
///     foo: i32,
/// }
///
/// impl PropertyStructure for ExampleProperty {
///     // ...
///   # type Value = i32;
///   # fn name(&self) -> String {
///   #     "example-property".to_owned()
///   # }
///   # fn description(&self) -> PropertyDescription<Self::Value> {
///   #     PropertyDescription::default()
///   # }
/// }
///
/// #[async_trait]
/// impl Property for BuiltExampleProperty {
///     // ...
/// }
/// ```
/// will expand to
/// ```
/// # use gateway_addon_rust::{prelude::*, property::{BuiltProperty, PropertyBuilder, PropertyStructure}};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleProperty {
///     foo: i32,
/// }
///
/// struct BuiltExampleProperty {
///     data: ExampleProperty,
///     property_handle: PropertyHandle<<ExampleProperty as PropertyStructure>::Value>,
/// }
///
/// impl BuiltProperty for BuiltExampleProperty {
///     type Value = <ExampleProperty as PropertyStructure>::Value;
///     fn property_handle(&self) -> &PropertyHandle<Self::Value> {
///         &self.property_handle
///     }
///     fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
///         &mut self.property_handle
///     }
/// }
///
/// impl PropertyBuilder for ExampleProperty {
///     type BuiltProperty = BuiltExampleProperty;
///     fn build(data: Self, property_handle: PropertyHandle<<ExampleProperty as PropertyStructure>::Value>) -> Self::BuiltProperty {
///         BuiltExampleProperty {
///             data,
///             property_handle,
///         }
///     }
/// }
///
/// impl Deref for BuiltExampleProperty {
///     type Target = ExampleProperty;
///     fn deref(&self) -> &Self::Target {
///         &self.data
///     }
/// }
///
/// impl DerefMut for BuiltExampleProperty {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.data
///     }
/// }
///
/// impl PropertyStructure for ExampleProperty {
///     // ...
///   # type Value = i32;
///   # fn name(&self) -> String {
///   #     "example-property".to_owned()
///   # }
///   # fn description(&self) -> PropertyDescription<Self::Value> {
///   #     PropertyDescription::default()
///   # }
/// }
///
/// #[async_trait]
/// impl Property for BuiltExampleProperty {
///     // ...
/// }
/// ```
pub use gateway_addon_rust_codegen::property;
