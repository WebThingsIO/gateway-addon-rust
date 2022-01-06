/// Use this on a struct to generate a built event around it, including useful impls.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::EventBuilder};
/// # use async_trait::async_trait;
/// #[event]
/// struct ExampleEvent {
///     foo: i32,
/// }
///
/// impl EventStructure for ExampleEvent {
///     // ...
///   # type Data = i32;
///   # fn name(&self) -> String {
///   #     "example-event".to_owned()
///   # }
///   # fn description(&self) -> EventDescription<Self::Data> {
///   #     EventDescription::default()
///   # }
/// }
///
/// #[async_trait]
/// impl Event for BuiltExampleEvent {
///     // ...
/// }
/// ```
/// will expand to
/// ```
/// # use gateway_addon_rust::{prelude::*, event::{BuiltEvent, EventBuilder, EventStructure}};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleEvent {
///     foo: i32,
/// }
///
/// struct BuiltExampleEvent {
///     data: ExampleEvent,
///     event_handle: EventHandle<<ExampleEvent as EventStructure>::Data>,
/// }
///
/// impl BuiltEvent for BuiltExampleEvent {
///     type Data = <ExampleEvent as EventStructure>::Data;
///     fn event_handle(&self) -> &EventHandle<Self::Data> {
///         &self.event_handle
///     }
///     fn event_handle_mut(&mut self) -> &mut EventHandle<Self::Data> {
///         &mut self.event_handle
///     }
/// }
///
/// impl EventBuilder for ExampleEvent {
///     type BuiltEvent = BuiltExampleEvent;
///     fn build(data: Self, event_handle: EventHandle<<ExampleEvent as EventStructure>::Data>) -> Self::BuiltEvent {
///         BuiltExampleEvent {
///             data,
///             event_handle,
///         }
///     }
/// }
///
/// impl Deref for BuiltExampleEvent {
///     type Target = ExampleEvent;
///     fn deref(&self) -> &Self::Target {
///         &self.data
///     }
/// }
///
/// impl DerefMut for BuiltExampleEvent {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.data
///     }
/// }
///
/// impl EventStructure for ExampleEvent {
///     // ...
///   # type Data = i32;
///   # fn name(&self) -> String {
///   #     "example-event".to_owned()
///   # }
///   # fn description(&self) -> EventDescription<Self::Data> {
///   #     EventDescription::default()
///   # }
/// }
///
/// #[async_trait]
/// impl Event for BuiltExampleEvent {
///     // ...
/// }
/// ```
pub use gateway_addon_rust_codegen::event;
