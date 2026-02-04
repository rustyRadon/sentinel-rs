use anyhow::Result;
use std::sync::Arc;
use crate::engine::SentinelNode;
use mdns_sd::{ServiceInfo, ServiceEvent};

pub async fn start_discovery(node: Arc<SentinelNode>, port: u16) -> Result<()> {
    let service_type = "_sentinel._tcp.local.";
    let instance_name = format!("node-{}", &node.identity.node_id()[..8]);
    
    // register node so others can see usss
    let my_info = ServiceInfo::new(
        service_type,
        &instance_name,
        &format!("{}.local.", instance_name),
        "127.0.0.1", 
        port,
        None,
    )?;
    
    node.mdns.register(my_info)?;

    // browse abi look for other nodes
    let receiver = node.mdns.browse(service_type)?;
    
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            if let ServiceEvent::ServiceResolved(info) = event {
                let addr = info.get_addresses().iter().next();
                if let Some(ip) = addr {
                    let full_addr = format!("{}:{}", ip, info.get_port());
                    
                    // only dial if we aren't already connected
                    if !node.peers.contains_key(&full_addr) && ip.to_string() != "0.0.0.0" {
                        let n = Arc::clone(&node);
                        tokio::spawn(async move {
                            if let Err(e) = n.dial_peer(full_addr).await {
                                //  fail discovery dials to avoid spamming the console
                                let _ = e; 
                            }
                        });
                    }
                }
            }
        }
    });

    Ok(())
}