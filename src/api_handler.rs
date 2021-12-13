/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use as_any::{AsAny, Downcast};
use async_trait::async_trait;
pub use webthings_gateway_ipc_types::{Request as ApiRequest, Response as ApiResponse};

#[async_trait]
pub trait ApiHandler: Send + Sync + AsAny + 'static {
    async fn on_unload(&mut self) {}
    async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
}

impl Downcast for dyn ApiHandler {}

pub struct NoopApiHandler;

impl NoopApiHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiHandler for NoopApiHandler {
    async fn handle_request(&mut self, _request: ApiRequest) -> Result<ApiResponse, String> {
        Err("No Api Handler registered".to_owned())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{ApiHandler, ApiRequest, ApiResponse};
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        pub ApiHandlerHelper {
            pub async fn on_unload(&mut self);
            pub async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
        }
    }

    pub struct MockApiHandler {
        pub api_handler_helper: MockApiHandlerHelper,
    }

    impl MockApiHandler {
        pub fn new() -> Self {
            Self {
                api_handler_helper: MockApiHandlerHelper::default(),
            }
        }
    }

    #[async_trait]
    impl ApiHandler for MockApiHandler {
        async fn on_unload(&mut self) {
            self.api_handler_helper.on_unload().await
        }

        async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String> {
            self.api_handler_helper.handle_request(request).await
        }
    }
}
