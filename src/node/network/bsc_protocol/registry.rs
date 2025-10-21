use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::sync::broadcast;

use reth_network_api::PeerId;

use super::stream::BscCommand;

/// Global registry of active BSC protocol senders per peer.
static REGISTRY: Lazy<RwLock<HashMap<PeerId, UnboundedSender<BscCommand>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Optional background task handle for EVN post-sync peer refresh.
static EVN_REFRESH_TASK: Lazy<RwLock<Option<JoinHandle<()>>>> =
    Lazy::new(|| RwLock::new(None));

/// Register a new peer's sender channel.
pub fn register_peer(peer: PeerId, tx: UnboundedSender<BscCommand>) {
    let guard = REGISTRY.write();
    match guard {
        Ok(mut g) => {
            g.insert(peer, tx);
        }
        Err(e) => {
            tracing::error!(target: "bsc::registry", error=%e, "Registry lock poisoned (register)");
        }
    }
}

/// Broadcast votes to all connected peers.
pub fn broadcast_votes(votes: Vec<crate::consensus::parlia::vote::VoteEnvelope>) {
    let votes_arc = Arc::new(votes);
    let mut to_remove: Vec<PeerId> = Vec::new();
    match REGISTRY.read() {
        Ok(guard) => {
            for (peer, tx) in guard.iter() {
                if tx.send(BscCommand::Votes(Arc::clone(&votes_arc))).is_err() {
                    to_remove.push(*peer);
                }
            }
        }
        Err(e) => {
            tracing::error!(target: "bsc::registry", error=%e, "Registry lock poisoned (broadcast)");
            return;
        }
    }

    if !to_remove.is_empty() {
        match REGISTRY.write() {
            Ok(mut guard) => {
                for peer in to_remove {
                    guard.remove(&peer);
                }
            }
            Err(e) => {
                tracing::error!(target: "bsc::registry", error=%e, "Registry lock poisoned (cleanup)");
            }
        }
    }
}

// Snapshot current connected peers (BSC protocol) by PeerId.
// Note: currently used only as part of internal EVN refresh; can be reinstated if needed.

/// Subscribe to EVN-armed notification and log-refresh current peers.
/// This helps post-sync peers reflect EVN policy locally. Remote peers
/// will pick up EVN on subsequent handshakes; this is a best-effort local refresh.
pub fn spawn_evn_refresh_listener() {
    // One-shot install only
    if let Ok(mut guard) = EVN_REFRESH_TASK.write() {
        if guard.is_some() { return; }

        // Subscribe to EVN armed broadcast channel
        let rx = crate::node::network::evn::subscribe_evn_armed();
        let handle = tokio::spawn(async move {
            let mut rx = rx;
            loop {
                match rx.recv().await {
                    Ok(()) => {
                        // On EVN arm, log the currently registered peers
                        let peers: Vec<PeerId> = match REGISTRY.read() {
                            Ok(g) => g.keys().copied().collect(),
                            Err(_) => Vec::new(),
                        };
                        tracing::info!(
                            target: "bsc::evn",
                            peer_count = peers.len(),
                            "EVN armed: refreshing EVN state for existing peers"
                        );
                        // Apply on-chain NodeIDs to current peers if available
                        let nodeids = crate::node::network::evn_peers::get_onchain_nodeids_set();
                        let mut marked = 0usize;
                        for p in peers {
                            let pid = p.to_string();
                            let pid_norm = crate::node::network::evn_peers::normalize_node_id_str(&pid);
                            if nodeids.contains(&pid_norm) {
                                crate::node::network::evn_peers::mark_evn_onchain(p);
                                marked += 1;
                            }
                        }
                        tracing::info!(target: "bsc::evn", marked, "Applied on-chain EVN NodeIDs to peers");

                        // Start periodic refresh every 60s to apply on-chain NodeIDs to existing peers
                        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
                        loop {
                            ticker.tick().await;
                            let peers: Vec<PeerId> = match REGISTRY.read() {
                                Ok(g) => g.keys().copied().collect(),
                                Err(_) => Vec::new(),
                            };
                            let nodeids = crate::node::network::evn_peers::get_onchain_nodeids_set();
                            let mut marked = 0usize;
                            for p in peers {
                                let pid = p.to_string();
                                let pid_norm = crate::node::network::evn_peers::normalize_node_id_str(&pid);
                                if nodeids.contains(&pid_norm) {
                                    crate::node::network::evn_peers::mark_evn_onchain(p);
                                    marked += 1;
                                }
                            }
                            tracing::debug!(target: "bsc::evn", marked, "Periodic EVN on-chain NodeIDs applied to peers");
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });
        *guard = Some(handle);
    }
}
