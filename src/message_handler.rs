use async_trait::async_trait;
use webthings_gateway_ipc_types::Message as IPCMessage;

#[doc(hidden)]
pub(crate) enum MessageResult {
    Continue,
    Terminate,
}

#[async_trait]
pub(crate) trait MessageHandler {
    async fn handle_message(&mut self, message: IPCMessage) -> Result<MessageResult, String>;
}
