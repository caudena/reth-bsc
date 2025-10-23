use alloy_primitives::Address;
use std::sync::{OnceLock, atomic::{AtomicBool, Ordering}};
use tokio::sync::broadcast;

/// EVN configuration
#[derive(Clone, Debug, Default)]
pub struct EvnConfig {
    pub enabled: bool,
    pub whitelist_nodeids: Vec<String>,
    pub proxyed_validators: Vec<Address>,
    pub nodeids_to_add: Vec<[u8; 32]>,
    pub nodeids_to_remove: Vec<[u8; 32]>,
    pub nodeids_nonce: Option<u64>,
}

/// Global EVN config
static GLOBAL_EVN_CONFIG: OnceLock<EvnConfig> = OnceLock::new();

/// Sets the global EVN config. Returns an error if already set.
pub fn set_global_evn_config(cfg: EvnConfig) -> Result<(), EvnConfig> { GLOBAL_EVN_CONFIG.set(cfg) }

/// Convenience: set only enabled flag.
pub fn set_global_evn_enabled(enabled: bool) -> Result<(), bool> {
    if let Some(cfg) = GLOBAL_EVN_CONFIG.get() {
        let mut new = cfg.clone();
        new.enabled = enabled;
        return GLOBAL_EVN_CONFIG.set(new).map_err(|_| enabled);
    }
    GLOBAL_EVN_CONFIG.set(EvnConfig { enabled, ..Default::default() }).map_err(|_| enabled)
}

/// Returns the global EVN config if set.
pub fn get_global_evn_config() -> Option<&'static EvnConfig> { GLOBAL_EVN_CONFIG.get() }

/// Returns true if EVN is enabled globally either via explicit global set or
/// via environment variable fallback.
pub fn is_evn_enabled() -> bool {
    if let Some(cfg) = GLOBAL_EVN_CONFIG.get() {
        return cfg.enabled;
    }
    // Fallback to environment var if global not initialized by CLI.
    std::env::var("BSC_EVN_ENABLED")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false)
}

/// Global EVN synced flag (armed after node is considered synced)
static EVN_SYNCED: AtomicBool = AtomicBool::new(false);
static EVN_ARMED_TX: OnceLock<broadcast::Sender<()>> = OnceLock::new();

/// Mark EVN as synced/armed. Once set to true it stays true.
pub fn set_evn_synced(synced: bool) {
    if synced {
        EVN_SYNCED.store(true, Ordering::Relaxed);
        // Broadcast EVN-armed event (create channel lazily)
        if let Some(tx) = EVN_ARMED_TX.get() {
            let _ = tx.send(());
        }
    } else {
        // keep it sticky-on to mirror geth behavior once synced; do not flip off
    }
}

/// Returns whether EVN is considered synced/armed.
pub fn is_evn_synced() -> bool { EVN_SYNCED.load(Ordering::Relaxed) }

/// EVN is ready only when enabled and synced (post-initial-sync)
pub fn is_evn_ready() -> bool { is_evn_enabled() && is_evn_synced() }

/// Returns a receiver for EVN armed notifications; creates the channel if missing.
pub fn subscribe_evn_armed() -> broadcast::Receiver<()> {
    let tx = EVN_ARMED_TX.get_or_init(|| broadcast::channel(4).0);
    tx.subscribe()
}
