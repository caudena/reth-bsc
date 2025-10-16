use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::mpsc::UnboundedSender;

use reth_network_api::PeerId;

use super::stream::BscCommand;

/// Global registry of active BSC protocol senders per peer.
static REGISTRY: Lazy<RwLock<HashMap<PeerId, UnboundedSender<BscCommand>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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
                if tx.send(BscCommand::SendVotes(Arc::clone(&votes_arc))).is_err() {
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
