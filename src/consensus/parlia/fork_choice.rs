use alloy_primitives::B256;
use reth_primitives::Header;
use crate::consensus::parlia::Snapshot;

/// BSC fork choice implementation that prioritizes fast finality over total difficulty.
/// This matches BSC's `ReorgNeededWithFastFinality` function.
pub struct BscForkChoice;

impl BscForkChoice {
    /// Determines if a reorganization is needed based on BSC's fast finality rules.
    /// 
    /// This implementation follows BSC's `ReorgNeededWithFastFinality` logic:
    /// 1. For PoSA consensus, check justified block numbers first
    /// 2. If justified numbers are equal, fall back to total difficulty comparison  
    /// 3. Prefer the chain with higher justified block number
    ///
    /// # Arguments
    /// * `current` - Current canonical head header
    /// * `incoming` - New header to potentially switch to
    /// * `current_snap` - Snapshot for current header (contains justification info)
    /// * `incoming_snap` - Snapshot for incoming header (contains justification info)
    /// * `is_plato_active` - Whether Plato hard fork (fast finality) is active
    ///
    /// # Returns
    /// * `Ok(true)` - Reorg needed, switch to incoming header
    /// * `Ok(false)` - No reorg needed, keep current header  
    /// * `Err(_)` - Error occurred during comparison
    pub fn reorg_needed_with_fast_finality(
        current: &Header,
        incoming: &Header,
        current_snap: Option<&Snapshot>,
        incoming_snap: Option<&Snapshot>,
        is_plato_active: bool,
    ) -> bool {
        // If Plato (fast finality) is not active, fall back to simple comparison
        if !is_plato_active {
            return Self::reorg_needed_simple(current, incoming);
        }

        let (incoming_justified, current_justified) = 
            Self::extract_justified_numbers(current_snap, incoming_snap);

        // Fast finality rule: prefer chain with higher justified block number
        if incoming_justified != current_justified {
            let should_reorg = incoming_justified > current_justified;
            
            // Log significant justified number changes (matching BSC behavior)
            if should_reorg && incoming.number <= current.number {
                tracing::info!(
                    "Chain found higher justified number: from_height={}, from_hash={:?}, from_justified={}, to_height={}, to_hash={:?}, to_justified={}",
                    current.number,
                    current.hash_slow(),
                    current_justified,
                    incoming.number, 
                    incoming.hash_slow(),
                    incoming_justified
                );
            }
            
            return should_reorg;
        }

        // If justified numbers are equal, fall back to traditional comparison
        Self::reorg_needed_simple(current, incoming)
    }

    /// Extract justified block numbers from snapshots.
    /// Returns (incoming_justified, current_justified) tuple.
    fn extract_justified_numbers(
        current_snap: Option<&Snapshot>,
        incoming_snap: Option<&Snapshot>,
    ) -> (u64, u64) {
        let incoming_justified = incoming_snap
            .map(|snap| snap.vote_data.target_number)
            .unwrap_or(0);
            
        let current_justified = current_snap
            .map(|snap| snap.vote_data.target_number)
            .unwrap_or(0);
            
        (incoming_justified, current_justified)
    }

    /// Simple reorg logic based on block number and timestamp.
    /// Used as fallback when justified numbers are equal or Plato is inactive.
    fn reorg_needed_simple(current: &Header, incoming: &Header) -> bool {
        match incoming.number.cmp(&current.number) {
            std::cmp::Ordering::Greater => true,  // Always accept longer chain
            std::cmp::Ordering::Less => false,    // Never accept shorter chain
            std::cmp::Ordering::Equal => {
                // Same height: prefer earlier timestamp (anti-selfish mining)
                incoming.timestamp < current.timestamp
            }
        }
    }

    /// Get justified number and hash for a given header via snapshot.
    /// This matches BSC's `GetJustifiedNumberAndHash` function.
    pub fn get_justified_number_and_hash(
        snap: Option<&Snapshot>,
    ) -> (u64, B256) {
        if let Some(snap) = snap {
            return (
                snap.vote_data.target_number,
                snap.vote_data.target_hash,
            );
        }
        
        // Return genesis block as fallback (matching BSC behavior)
        (0, B256::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_header(number: u64, timestamp: u64) -> Header {
        Header {
            number,
            timestamp,
            ..Default::default()
        }
    }

    fn create_snapshot_with_justified(justified_number: u64, justified_hash: B256) -> Snapshot {
        use crate::consensus::parlia::{VoteData};
        
        let vote_data = VoteData {
            source_number: 0,
            source_hash: B256::ZERO,
            target_number: justified_number,
            target_hash: justified_hash,
        };
        
        Snapshot {
            vote_data,
            ..Default::default()
        }
    }

    #[test]
    fn test_higher_justified_number_triggers_reorg() {
        let current = create_header(100, 1000);
        let incoming = create_header(99, 1001); // Lower height but higher justified
        
        let current_snap = create_snapshot_with_justified(50, B256::ZERO);
        let incoming_snap = create_snapshot_with_justified(60, B256::ZERO);
        
        let result = BscForkChoice::reorg_needed_with_fast_finality(
            &current,
            &incoming,
            Some(&current_snap),
            Some(&incoming_snap),
            true, // Plato active
        );
        
        assert!(result, "Should reorg to chain with higher justified number");
    }

    #[test]
    fn test_equal_justified_falls_back_to_simple_logic() {
        let current = create_header(100, 1000);
        let incoming = create_header(101, 1001); // Higher height
        
        let current_snap = create_snapshot_with_justified(50, B256::ZERO);
        let incoming_snap = create_snapshot_with_justified(50, B256::ZERO); // Same justified
        
        let result = BscForkChoice::reorg_needed_with_fast_finality(
            &current,
            &incoming,
            Some(&current_snap),
            Some(&incoming_snap),
            true, // Plato active
        );
        
        assert!(result, "Should reorg to longer chain when justified numbers equal");
    }

    #[test]
    fn test_plato_inactive_uses_simple_logic() {
        let current = create_header(100, 1000);
        let incoming = create_header(99, 1001);
        
        let current_snap = create_snapshot_with_justified(10, B256::ZERO);
        let incoming_snap = create_snapshot_with_justified(20, B256::ZERO);
        
        let result = BscForkChoice::reorg_needed_with_fast_finality(
            &current,
            &incoming,
            Some(&current_snap),
            Some(&incoming_snap),
            false, // Plato inactive
        );
        
        assert!(!result, "Should not reorg to shorter chain when Plato inactive");
    }

    #[test]
    fn test_same_height_prefers_earlier_timestamp() {
        let current = create_header(100, 1001);
        let incoming = create_header(100, 1000); // Same height, earlier timestamp
        
        let result = BscForkChoice::reorg_needed_simple(&current, &incoming);
        assert!(result, "Should prefer earlier timestamp at same height");
    }
}