use std::sync::OnceLock;
use reth_discv4::NodeRecord;
use eyre::Result;

/// Global storage for bootnode override
static BOOTNODE_OVERRIDE: OnceLock<Option<Vec<NodeRecord>>> = OnceLock::new();

/// Set the global bootnode override
pub fn set_bootnode_override(bootnodes: Option<Vec<NodeRecord>>) -> Result<()> {
    BOOTNODE_OVERRIDE.set(bootnodes)
        .map_err(|_| eyre::eyre!("Bootnode override already set"))?;
    
    if let Some(ref nodes) = BOOTNODE_OVERRIDE.get().unwrap() {
        tracing::info!("Bootnode override set with {} nodes", nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            tracing::info!("Bootnode {}: {}", i + 1, node);
        }
    } else {
        tracing::info!("Bootnode override disabled - will use default bootnodes");
    }
    
    Ok(())
}

/// Get the global bootnode override
pub fn get_bootnode_override() -> &'static Option<Vec<NodeRecord>> {
    BOOTNODE_OVERRIDE.get().unwrap_or(&None)
}

/// Check if bootnode override is active
pub fn has_bootnode_override() -> bool {
    BOOTNODE_OVERRIDE.get().map_or(false, |nodes| nodes.is_some())
}