use crate::engine::SentinelNode;
use anyhow::Result;
use mdns_sd::{ServiceEvent, ServiceInfo};
use std::sync::Arc;

impl SentinelNode {
    pub fn start_discovery(self: &Arc<Self>, port: u16) -> Result<()> {
        let mdns = self.mdns.clone();
        let node_id = self.identity.node_id();
        let service_type = "_sentinel._tcp.local.";
        
        let instance_name = format!("{}.sentinel", node_id);
        let my_service = ServiceInfo::new(
            service_type, &instance_name, "sentinel-node.local.", "", port, None,
        )?;
        mdns.register(my_service)?;

        let receiver = mdns.browse(service_type)?;
        let node_inner = Arc::clone(self);

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                if let ServiceEvent::ServiceResolved(info) = event {
                    let name = info.get_fullname();
                    
                    if name.contains(&node_id) { continue; }

                    let addr_list = info.get_addresses().clone();
                    let port = info.get_port();

                    if let Some(ip) = addr_list.iter().next() {
                        let target = format!("{}:{}", ip, port);
                        println!("mDNS: Discovered Peer at {}", target);
                        
                        let node_to_dial = Arc::clone(&node_inner);
                        tokio::spawn(async move {
                            if let Err(e) = node_to_dial.dial_peer(target).await {
                                eprintln!("Dial error: {}", e);
                            }
                        });
                    }
                }
            }
        });
        Ok(())
    }
}