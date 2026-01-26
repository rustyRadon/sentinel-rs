use std::collections::HashMap;
use std::sync::Arc;
use tracing::{warn, debug};

use sentinel_protocol::frame::Frame;
use sentinel_protocol::commands::CommandHandler;
use crate::handlers::{
    SysInfoHandler, FileUploadHandler, ChatHandler, ScreenshotHandler
};

pub struct CommandRouter {
    handlers: HashMap<u8, Arc<Box<dyn CommandHandler>>>,
}

impl CommandRouter {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: u8, handler: impl CommandHandler + 'static) {
        self.handlers.insert(id, Arc::new(Box::new(handler)));
    }

    pub fn with_default_commands() -> Self {
        let mut router = Self::new();

        // 0x01: System Information
        router.register(0x01, SysInfoHandler);

        // 0x03: Secure File Upload
        router.register(0x03, FileUploadHandler);

        // 0x05: Remote Chat
        router.register(0x05, ChatHandler);

        // 0x07: Screenshot Capture
        router.register(0x07, ScreenshotHandler);

        router
    }

    pub async fn dispatch(&self, frame: Frame) -> Result<Option<Frame>, anyhow::Error> {
        let cmd_id = frame.flags();
        debug!("Routing command ID: 0x{:02X}", cmd_id);

        if let Some(handler) = self.handlers.get(&cmd_id) {
            handler.handle(frame).await.map_err(|e| anyhow::anyhow!("Handler error: {:?}", e))
        } else {
            warn!("Received unknown command ID: 0x{:02X}", cmd_id);
            Err(anyhow::anyhow!("Unknown Command ID: 0x{:02X}", cmd_id))
        }
    }
}