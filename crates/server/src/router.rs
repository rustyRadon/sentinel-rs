use std::collections::HashMap;
use std::sync::Arc;
use tracing::{warn, debug};

use sentinel_protocol::frame::Frame;
use sentinel_protocol::commands::CommandHandler;
use crate::handlers::{SysInfoHandler, FileUploadHandler};

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

        // 0x01: System Information (Uptime, Version, etc.)
        router.register(0x01, SysInfoHandler);

        // 0x03: Secure File Upload (Binary exfiltration)
        router.register(0x03, FileUploadHandler);

        router
    }

    /// Dispatches the frame to the registered handler based on the first byte of the payload.
    /// Returns Ok(Some(Frame)) for requests, Ok(None) for fire-and-forget, or Err for protocol violations.
    pub async fn dispatch(&self, frame: Frame) -> Result<Option<Frame>, anyhow::Error> {
        let payload = frame.payload();

        if payload.is_empty() {
            warn!("Received frame with empty payload; cannot route.");
            return Err(anyhow::anyhow!("Payload missing Command ID"));
        }

        let cmd_id = payload[0];
        debug!("Routing command ID: 0x{:02X}", cmd_id);

        if let Some(handler) = self.handlers.get(&cmd_id) {
            handler.handle(frame).await.map_err(|e| anyhow::anyhow!("Handler error: {:?}", e))
        } else {
            warn!("Received unknown command ID: 0x{:02X}", cmd_id);
            Err(anyhow::anyhow!("Unknown Command ID: 0x{:02X}", cmd_id))
        }
    }
}