use std::sync::Arc;

use crate::{
    node::{engine_api::payload::BscPayloadTypes, BscNode},
    BscBlock, BscPrimitives,
};
use alloy_eips::eip7685::Requests;
use alloy_primitives::U256;
use reth::{
    api::FullNodeTypes,
    builder::{components::PayloadServiceBuilder, BuilderContext},
    payload::{PayloadBuilderHandle, PayloadServiceCommand},
    transaction_pool::TransactionPool,
};
use reth_evm::ConfigureEvm;
use reth_payload_primitives::BuiltPayload;
use reth_primitives::SealedBlock;
use tokio::sync::{broadcast, mpsc};
use tracing::warn;

/// Built payload for BSC. This is similar to [`EthBuiltPayload`] but without sidecars as those
/// included into [`BscBlock`].
#[derive(Debug, Clone)]
pub struct BscBuiltPayload {
    /// The built block
    pub(crate) block: Arc<SealedBlock<BscBlock>>,
    /// The fees of the block
    pub(crate) fees: U256,
    /// The requests of the payload
    pub(crate) requests: Option<Requests>,
}

impl BuiltPayload for BscBuiltPayload {
    type Primitives = BscPrimitives;

    fn block(&self) -> &SealedBlock<BscBlock> {
        self.block.as_ref()
    }

    fn fees(&self) -> U256 {
        self.fees
    }

    fn requests(&self) -> Option<Requests> {
        self.requests.clone()
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct BscPayloadServiceBuilder;

/// Mining Service that handles block production for BSC
pub struct BscMiner<Pool, Provider> {
    pool: Pool,
    provider: Provider,
    snapshot_provider: Arc<dyn SnapshotProvider + Send + Sync>,
    validator_address: Address,
    chain_spec: Arc<crate::chainspec::BscChainSpec>,
    parlia: Arc<crate::consensus::parlia::Parlia<crate::chainspec::BscChainSpec>>,
    signing_key: Option<SigningKey>,
    mining_config: MiningConfig,
}

impl<Pool, Provider> BscMiner<Pool, Provider>
where
    Pool: TransactionPool + Clone + 'static,
    Provider: HeaderProvider<Header = alloy_consensus::Header> + BlockNumReader + Clone + Send + Sync + 'static,
{
    pub fn new(
        pool: Pool,
        provider: Provider,
        snapshot_provider: Arc<dyn SnapshotProvider + Send + Sync>,
        chain_spec: Arc<crate::chainspec::BscChainSpec>,
        mining_config: MiningConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Validate mining configuration
        mining_config.validate()?;
        
        // We'll derive and trust the validator address from the configured signing key when possible.
        // If not available, fall back to configured address (may be ZERO when disabled).
        let mut validator_address = mining_config.validator_address.unwrap_or(Address::ZERO);
        
        // Load signing key if mining is enabled
        let signing_key = if mining_config.is_mining_enabled() {
            let key = if let Some(keystore_path) = &mining_config.keystore_path {
                let password = mining_config.keystore_password.as_deref().unwrap_or("");
                keystore::load_private_key_from_keystore(keystore_path, password)?
            } else if let Some(hex_key) = &mining_config.private_key_hex {
                keystore::load_private_key_from_hex(hex_key)?
            } else {
                return Err("No signing key configured".into());
            };
            
            // Derive validator address from the signing key and prefer it
            let derived_address = keystore::get_validator_address(&key);
            if derived_address != validator_address {
                if validator_address != Address::ZERO {
                    warn!("Validator address mismatch: configured={}, derived={}", validator_address, derived_address);
                }
                info!("Using derived address from private key: {}", derived_address);
                validator_address = derived_address;
            }
            
            Some(key)
        } else {
            None
        };

        Ok(Self {
            pool,
            provider,
            snapshot_provider,
            validator_address,
            chain_spec: chain_spec.clone(),
            parlia: Arc::new(crate::consensus::parlia::Parlia::new(chain_spec, 200)),
            signing_key,
            mining_config,
        })
    }

    /// Start the PoA mining loop
    pub async fn start_mining(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.mining_config.is_mining_enabled() {
            info!("Mining is disabled in configuration");
            return Ok(());
        }
        
        info!("Starting BSC mining service for validator: {}", self.validator_address);
        
        // Mining interval from config or default
        let interval_ms = self.mining_config.mining_interval_ms.unwrap_or(500);
        let mut mining_interval = interval(Duration::from_millis(interval_ms));
        
        loop {
            mining_interval.tick().await;
            
            if let Err(e) = self.try_mine_block().await {
                debug!("Mining attempt failed: {}", e);
                // Continue mining loop even if individual attempts fail
            }
        }
    }

    /// Attempt to mine a block if conditions are met
    async fn try_mine_block(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get current head block from chain state
        let current_block_number = self.provider.best_block_number()?;
        let head_header = self.provider.header_by_number(current_block_number)?
            .ok_or("Head block header not found")?;
        
        // Create sealed header for the current head block
        use alloy_primitives::keccak256;
        let head_hash = keccak256(alloy_rlp::encode(&head_header));
        let head = reth_primitives::SealedHeader::new(head_header, head_hash);
        
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let parent_number = head.number();
        
        // Get snapshot for parent block to check authorization
        let snapshot = self.snapshot_provider.snapshot(parent_number)
            .ok_or("No snapshot available for parent block")?;
        
        // Check if we're authorized to mine
        if !snapshot.validators.contains(&self.validator_address) {
            return Err(format!("Not authorized validator: {}", self.validator_address).into());
        }
        
        // Check if we signed recently (avoid signing too frequently)
        if snapshot.sign_recently(self.validator_address) {
            return Err("Signed recently, must wait for others".into());
        }
        
        // Calculate when we should mine based on turn and backoff
        let next_block_time = self.calculate_next_block_time(&head, &snapshot, current_time)?;
        
        if current_time < next_block_time {
            return Err(format!("Too early to mine, wait until {next_block_time}").into());
        }
        
        info!("Mining new block on top of block {}", parent_number);
        
        // Build and seal the block
        self.mine_block_now(&head).await
    }

    /// Calculate the optimal time to mine the next block
    fn calculate_next_block_time(
        &self,
        parent: &reth_primitives::SealedHeader,
        snapshot: &crate::consensus::parlia::Snapshot,
        _current_time: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        use crate::consensus::parlia::constants::DIFF_NOTURN;

        // Scheduled next time in ms: parent time (ms) + period (ms)
        let parent_ts_ms = calculate_millisecond_timestamp(parent.header());
        let period_ms = snapshot.block_interval;
        let scheduled_ms = parent_ts_ms + period_ms;

        // Candidate header for backoff calculation
        let mut candidate = alloy_consensus::Header::default();
        candidate.number = parent.number() + 1;
        candidate.timestamp = scheduled_ms / 1000; // seconds part for header
        candidate.beneficiary = self.validator_address;
        candidate.difficulty = U256::from(DIFF_NOTURN);

        // Compute final delay using Parlia helper (ms)
        let left_over_ms: u64 = 0; // reserved time for finalize
        let delay_ms = self.parlia.compute_delay_with_backoff(snapshot, parent.header(), &candidate, left_over_ms);

        // Final time in seconds (ceil ms)
        let target_ms = scheduled_ms + delay_ms;
        let target_secs = (target_ms + 999) / 1000;
        Ok(target_secs)
    }

    /// Mine a block immediately
    async fn mine_block_now(
        &self,
        parent: &reth_primitives::SealedHeader,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Build block header
        let mut header = alloy_consensus::Header {
            parent_hash: parent.hash(),
            number: parent.number() + 1,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            beneficiary: self.validator_address,
            gas_limit: parent.gas_limit(),
            extra_data: Bytes::from(vec![0u8; 32 + 65]), // Vanity + seal placeholder
            difficulty: self.calculate_difficulty(parent)?,
            ..Default::default()
        };
        
        // Collect transactions from the pool
        let transactions = self.collect_transactions(&header).await?;
        
        // Calculate gas used and other header fields
        header.gas_used = transactions.iter().map(|tx| tx.gas_limit()).sum();
        // TODO: Calculate proper transaction root
        header.transactions_root = alloy_primitives::keccak256(alloy_rlp::encode(&transactions));
        
        // Create block body
        let body = crate::BscBlockBody {
            inner: reth_primitives::BlockBody {
                transactions,
                ommers: Vec::new(),
                withdrawals: None,
            },
            sidecars: None,
        };
        
        // Create unsealed block
        let block = BscBlock { header, body };
        
        // Seal the block using Parlia consensus
        let signing_key: SigningKey = self.signing_key.clone()
            .ok_or("No signing key available for block sealing")?;

        // SealBlock init
        let seal_block = SealBlock::new(
            self.snapshot_provider.clone(),
            self.chain_spec.clone(),
            signing_key,
        );

        match seal_block.seal(block) {
            Ok(sealed_block) => {
                info!("Successfully mined block {}", sealed_block.number());
                // TODO: Submit sealed block to engine API or import directly
                self.submit_block(sealed_block).await?
            },
            Err(e) => {
                error!("Failed to seal block: {}", e);
                return Err(e.into());
            }
        }
        
        Ok(())
    }

    /// Calculate difficulty for the new block
    fn calculate_difficulty(
        &self,
        parent: &reth_primitives::SealedHeader,
    ) -> Result<U256, Box<dyn std::error::Error + Send + Sync>> {
        use crate::consensus::parlia::constants::{DIFF_INTURN, DIFF_NOTURN};
        
        let snapshot = self.snapshot_provider.snapshot(parent.number())
            .ok_or("No snapshot available")?;
        
        let difficulty = if snapshot.is_inturn(self.validator_address) {
            DIFF_INTURN
        } else {
            DIFF_NOTURN
        };
        
        Ok(U256::from(difficulty))
    }

    /// Collect transactions from the transaction pool
    async fn collect_transactions(
        &self,
        header: &alloy_consensus::Header,
    ) -> Result<Vec<TransactionSigned>, Box<dyn std::error::Error + Send + Sync>> {
        let transactions = Vec::new();
        let mut gas_used = 0u64;
        let gas_limit = header.gas_limit();
        
        // Get best transactions from pool
        let best_txs = self.pool.best_transactions();
        
        // Collect transactions until we hit gas limit
        for pooled_tx in best_txs {
            let tx = &pooled_tx.transaction;
            if gas_used + tx.gas_limit() > gas_limit {
                break;
            }
            gas_used += tx.gas_limit();
            // For now, skip transaction collection - focus on core mining logic
            // TODO: Implement proper transaction cloning based on transaction type
        }
        
        debug!("Collected {} transactions for block, gas used: {}", transactions.len(), gas_used);
        Ok(transactions)
    }

    /// Submit the sealed block (placeholder for now)
    async fn submit_block(
        &self,
        _sealed_block: SealedBlock<BscBlock>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement block submission to engine API
        // This would typically involve:
        // 1. Converting to execution payload format
        // 2. Submitting via engine API or importing directly
        warn!("Block submission not yet implemented");
        Ok(())
    }
}

impl<Node, Pool, Evm> PayloadServiceBuilder<Node, Pool, Evm> for BscPayloadServiceBuilder
where
    Node: FullNodeTypes<Types = BscNode>,
    Pool: TransactionPool,
    Evm: ConfigureEvm,
{
    async fn spawn_payload_builder_service(
        self,
        ctx: &BuilderContext<Node>,
        _pool: Pool,
        _evm_config: Evm,
    ) -> eyre::Result<PayloadBuilderHandle<BscPayloadTypes>> {
        let (tx, mut rx) = mpsc::unbounded_channel();

        ctx.task_executor().spawn_critical("payload builder", async move {
            let mut subscriptions = Vec::new();

            while let Some(message) = rx.recv().await {
                match message {
                    PayloadServiceCommand::Subscribe(tx) => {
                        let (events_tx, events_rx) = broadcast::channel(100);
                        // Retain senders to make sure that channels are not getting closed
                        subscriptions.push(events_tx);
                        let _ = tx.send(events_rx);
                    }
                    message => warn!(?message, "Noop payload service received a message"),
                }
            }
        });

        Ok(PayloadBuilderHandle::new(tx))
    }
}

// impl From<EthBuiltPayload> for BscBuiltPayload {
//     fn from(value: EthBuiltPayload) -> Self {
//         let EthBuiltPayload { id, block, fees, sidecars, requests } = value;
//         BscBuiltPayload {
//             id,
//             block: block.into(),
//             fees,
//             requests,
//         }
//     }
// }

// pub struct BscPayloadBuilder<Inner> {
//     inner: Inner,
// }

// impl<Inner> PayloadBuilder for BscPayloadBuilder<Inner>
// where
//     Inner: PayloadBuilder<BuiltPayload = EthBuiltPayload>,
// {
//     type Attributes = Inner::Attributes;
//     type BuiltPayload = BscBuiltPayload;
//     type Error = Inner::Error;

//     fn try_build(
//         &self,
//         args: BuildArguments<Self::Attributes, Self::BuiltPayload>,
//     ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
//         let outcome = self.inner.try_build(args)?;
//     }
// }
