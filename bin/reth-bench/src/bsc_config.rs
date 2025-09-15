//! BSC-specific configuration and constants

/// BSC Mainnet chain ID
pub const BSC_MAINNET_CHAIN_ID: u64 = 56;

/// BSC Testnet chain ID  
pub const BSC_TESTNET_CHAIN_ID: u64 = 97;

/// Check if a chain ID is BSC
pub fn is_bsc_chain(chain_id: u64) -> bool {
    chain_id == BSC_MAINNET_CHAIN_ID || chain_id == BSC_TESTNET_CHAIN_ID
}

/// Get BSC network name
pub fn bsc_network_name(chain_id: u64) -> &'static str {
    match chain_id {
        BSC_MAINNET_CHAIN_ID => "BSC Mainnet",
        BSC_TESTNET_CHAIN_ID => "BSC Testnet", 
        _ => "Unknown BSC Network",
    }
}