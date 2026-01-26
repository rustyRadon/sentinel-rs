use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;
use sysinfo::System;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
static SYS: Lazy<Arc<Mutex<System>>> = Lazy::new(|| {
    let mut s = System::new_all();
    s.refresh_all();
    Arc::new(Mutex::new(s))
});

pub struct SysInfoHandler;

#[async_trait]
impl CommandHandler for SysInfoHandler {
    async fn handle(&self, _frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        let mut s = SYS.lock().unwrap();
        s.refresh_cpu();
        s.refresh_memory();

        let load = s.global_cpu_info().cpu_usage();
        let total_mem = s.total_memory() / 1024 / 1024;
        let used_mem = s.used_memory() / 1024 / 1024;

        let stats = format!("CPU: {:.1}% | Mem: {}/{} MB", load, used_mem, total_mem);
        
        let response = Frame::new(1, 0x01, Bytes::from(stats))?;
        Ok(Some(response))
    }
}