/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{action::Input, ActionDescription, ActionHandle};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;

use jsonschema::JSONSchema;

use webthings_gateway_ipc_types::Action as FullActionDescription;

/// A trait used to specify the structure and behaviour of a WoT action.
///
/// Defines how to react on gateway requests.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, action::NoInput};
/// # use async_trait::async_trait;
/// struct ExampleAction();
///
/// #[async_trait]
/// impl Action for ExampleAction {
///     type Input = NoInput;
///
///     fn name(&self) -> String {
///         "example-action".to_owned()
///     }
///
///     fn description(&self) -> ActionDescription<Self::Input> {
///         ActionDescription::default()
///     }
///
///     async fn perform(
///         &mut self,
///         mut action_handle: ActionHandle<Self::Input>,
///     ) -> Result<(), String> {
///         action_handle.start();
///         log::debug!("Performing example-action");
///         action_handle.finish();
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Action: Send + Sync + 'static {
    /// Type of [input][Input] this action expects.
    type Input: Input;

    /// Name of the action.
    fn name(&self) -> String;

    /// [WoT description][ActionDescription] of the action.
    fn description(&self) -> ActionDescription<Self::Input>;

    /// Called when this action has been started through the gateway.
    ///
    /// If action execution may take a while, don't block this function.
    ///
    /// Don't forget to call `action_handle.start()` and `action_handle.finish()`.
    async fn perform(&mut self, _action_handle: ActionHandle<Self::Input>) -> Result<(), String>;

    /// Called when this action has been canceled through the gateway.
    async fn cancel(&mut self, _action_id: String) -> Result<(), String> {
        Err("Action does not implement canceling".to_owned())
    }

    #[doc(hidden)]
    fn full_description(&self) -> FullActionDescription {
        self.description().into_full_description()
    }

    #[doc(hidden)]
    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<serde_json::Value>,
    ) -> Result<(), String> {
        if let Some(ref input_schema) = self.description().input {
            let schema = JSONSchema::compile(input_schema).map_err(|err| {
                format!(
                    "Failed to parse input schema for action {:?}: {:?}",
                    self.name(),
                    err
                )
            })?;
            schema.validate(&action_handle.input).map_err(|err| {
                format!(
                    "Failed to validate input for action {:?}: {:?}",
                    self.name(),
                    err.collect::<Vec<_>>()
                )
            })?;
        }
        let input = Self::Input::deserialize(action_handle.input.clone())
            .map_err(|err| format!("Could not deserialize input: {:?}", err))?;
        self.perform(ActionHandle::new(
            action_handle.client,
            action_handle.device,
            action_handle.plugin_id,
            action_handle.adapter_id,
            action_handle.device_id,
            action_handle.name,
            action_handle.id,
            input,
            action_handle.input,
        ))
        .await
    }
}

/// An object safe variant of [Action].
///
/// Auto-implemented for all objects which implement the [Action] trait.  **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [Action] implementation.
///
/// This can (in contrast to the [Action] trait) be used to store objects for dynamic dispatch.
#[async_trait]
pub trait ActionBase: Send + Sync + AsAny + 'static {
    /// Name of the action.
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> FullActionDescription;

    #[doc(hidden)]
    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<serde_json::Value>,
    ) -> Result<(), String>;

    #[doc(hidden)]
    async fn cancel(&mut self, action_id: String) -> Result<(), String>;
}

impl Downcast for dyn ActionBase {}

#[async_trait]
impl<T: Action> ActionBase for T {
    fn name(&self) -> String {
        <T as Action>::name(self)
    }

    fn full_description(&self) -> FullActionDescription {
        <T as Action>::full_description(self)
    }

    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<serde_json::Value>,
    ) -> Result<(), String> {
        <T as Action>::check_and_perform(self, action_handle).await
    }

    async fn cancel(&mut self, action_id: String) -> Result<(), String> {
        <T as Action>::cancel(self, action_id).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{action::Input, Action, ActionDescription, ActionHandle};
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        pub ActionHelper<T: Input> {
            pub fn perform(&mut self, action_handle: ActionHandle<T>) -> Result<(), String>;
            pub fn cancel(&mut self, action_id: String) -> Result<(), String>;
        }
    }

    pub struct MockAction<T: Input> {
        action_name: String,
        pub action_helper: MockActionHelper<T>,
    }

    impl<T: Input> MockAction<T> {
        pub fn new(action_name: String) -> Self {
            Self {
                action_name,
                action_helper: MockActionHelper::new(),
            }
        }
    }

    #[async_trait]
    impl<T: Input> Action for MockAction<T> {
        type Input = T;

        fn name(&self) -> String {
            self.action_name.to_owned()
        }

        fn description(&self) -> ActionDescription<Self::Input> {
            ActionDescription::default()
        }

        async fn perform(
            &mut self,
            action_handle: ActionHandle<Self::Input>,
        ) -> Result<(), String> {
            assert!(action_handle.device.upgrade().is_some());
            self.action_helper.perform(action_handle)
        }

        async fn cancel(&mut self, action_id: String) -> Result<(), String> {
            self.action_helper.cancel(action_id)
        }
    }
}
