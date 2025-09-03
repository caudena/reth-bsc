//! This is an extension trait for any provider that implements the engine API, to wait for a VALID
//! response. This is useful for benchmarking, as it allows us to wait for a payload to be valid
//! before sending additional calls.

use alloy_eips::eip7685::Requests;
use alloy_provider::{ext::EngineApi, network::AnyRpcBlock, Network, Provider};
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadInputV2, ForkchoiceState, ForkchoiceUpdated,
    PayloadAttributes, PayloadStatus,
};
use alloy_transport::TransportResult;
use op_alloy_rpc_types_engine::OpExecutionPayloadV4;
use reth_node_api::EngineApiMessageVersion;
use tracing::error;

/// An extension trait for providers that implement the engine API, to wait for a VALID response.
#[async_trait::async_trait]
pub trait EngineApiValidWaitExt<N>: Send + Sync {
    /// Calls `engine_forkChoiceUpdatedV1` with the given [`ForkchoiceState`] and optional
    /// [`PayloadAttributes`], and waits until the response is VALID.
    async fn fork_choice_updated_v1_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Calls `engine_forkChoiceUpdatedV2` with the given [`ForkchoiceState`] and optional
    /// [`PayloadAttributes`], and waits until the response is VALID.
    async fn fork_choice_updated_v2_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Calls `engine_forkChoiceUpdatedV3` with the given [`ForkchoiceState`] and optional
    /// [`PayloadAttributes`], and waits until the response is VALID.
    async fn fork_choice_updated_v3_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;
}

#[async_trait::async_trait]
impl<N, P> EngineApiValidWaitExt<N> for P
where
    N: Network,
    P: Provider<N> + EngineApi<N>,
{
    async fn fork_choice_updated_v1_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        let mut status =
            self.fork_choice_updated_v1(fork_choice_state, payload_attributes.clone()).await?;

        while !status.is_valid() {
            if status.is_invalid() {
                error!(
                    ?status,
                    ?fork_choice_state,
                    ?payload_attributes,
                    "Invalid forkchoiceUpdatedV1 message",
                );
                panic!("Invalid forkchoiceUpdatedV1: {status:?}");
            }
            if status.is_syncing() {
                tracing::warn!("Node is syncing, waiting 200ms before retry...");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // Continue the loop to retry
            }
            status =
                self.fork_choice_updated_v1(fork_choice_state, payload_attributes.clone()).await?;
        }

        Ok(status)
    }

    async fn fork_choice_updated_v2_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        let mut status =
            self.fork_choice_updated_v2(fork_choice_state, payload_attributes.clone()).await?;

        while !status.is_valid() {
            if status.is_invalid() {
                error!(
                    ?status,
                    ?fork_choice_state,
                    ?payload_attributes,
                    "Invalid forkchoiceUpdatedV2 message",
                );
                panic!("Invalid forkchoiceUpdatedV2: {status:?}");
            }
            if status.is_syncing() {
                tracing::warn!("Node is syncing, waiting 200ms before retry...");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // Continue the loop to retry
            }
            status =
                self.fork_choice_updated_v2(fork_choice_state, payload_attributes.clone()).await?;
        }

        Ok(status)
    }

    async fn fork_choice_updated_v3_wait(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        let mut status =
            self.fork_choice_updated_v3(fork_choice_state, payload_attributes.clone()).await?;

        while !status.is_valid() {
            if status.is_invalid() {
                error!(
                    ?status,
                    ?fork_choice_state,
                    ?payload_attributes,
                    "Invalid forkchoiceUpdatedV3 message",
                );
                panic!("Invalid forkchoiceUpdatedV3: {status:?}");
            }
            if status.is_syncing() {
                tracing::warn!("Node is syncing, waiting 200ms before retry...");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // Continue the loop to retry
            }
            status =
                self.fork_choice_updated_v3(fork_choice_state, payload_attributes.clone()).await?;
        }

        Ok(status)
    }
}

pub(crate) fn block_to_new_payload(
    block: AnyRpcBlock,
    is_optimism: bool,
    chain_id: u64,
) -> eyre::Result<(EngineApiMessageVersion, serde_json::Value)> {
    use crate::bsc_config;
    
    let block = block
        .into_inner()
        .map_header(|header| header.map(|h| h.into_header_with_defaults()))
        .try_map_transactions(|tx| {
            // try to convert unknowns into op type so that we can also support optimism
            tx.try_into_either::<op_alloy_consensus::OpTxEnvelope>()
        })?
        .into_consensus();

    // Convert to execution payload
    let (payload, sidecar) = ExecutionPayload::from_block_slow(&block);
    
    // Force V1 for BSC networks
    if bsc_config::is_bsc_chain(chain_id) {
        let v1_payload = match payload {
            ExecutionPayload::V1(payload) => payload,
            ExecutionPayload::V2(payload) => payload.payload_inner,
            ExecutionPayload::V3(payload) => {
                // Convert V3 to V1 by removing V3-specific fields
                alloy_rpc_types_engine::ExecutionPayloadV1 {
                    parent_hash: payload.payload_inner.payload_inner.parent_hash,
                    fee_recipient: payload.payload_inner.payload_inner.fee_recipient,
                    state_root: payload.payload_inner.payload_inner.state_root,
                    receipts_root: payload.payload_inner.payload_inner.receipts_root,
                    logs_bloom: payload.payload_inner.payload_inner.logs_bloom,
                    prev_randao: payload.payload_inner.payload_inner.prev_randao,
                    block_number: payload.payload_inner.payload_inner.block_number,
                    gas_limit: payload.payload_inner.payload_inner.gas_limit,
                    gas_used: payload.payload_inner.payload_inner.gas_used,
                    timestamp: payload.payload_inner.payload_inner.timestamp,
                    extra_data: payload.payload_inner.payload_inner.extra_data,
                    base_fee_per_gas: payload.payload_inner.payload_inner.base_fee_per_gas,
                    block_hash: payload.payload_inner.payload_inner.block_hash,
                    transactions: payload.payload_inner.payload_inner.transactions,
                }
            }
        };
        return Ok((EngineApiMessageVersion::V1, serde_json::to_value((v1_payload,))?));
    }

    let (version, params) = match payload {
        ExecutionPayload::V3(payload) => {
            let cancun = sidecar.cancun().unwrap();

            if let Some(prague) = sidecar.prague() {
                if is_optimism {
                    (
                        EngineApiMessageVersion::V4,
                        serde_json::to_value((
                            OpExecutionPayloadV4 {
                                payload_inner: payload,
                                withdrawals_root: block.withdrawals_root.unwrap(),
                            },
                            cancun.versioned_hashes.clone(),
                            cancun.parent_beacon_block_root,
                            Requests::default(),
                        ))?,
                    )
                } else {
                    (
                        EngineApiMessageVersion::V4,
                        serde_json::to_value((
                            payload,
                            cancun.versioned_hashes.clone(),
                            cancun.parent_beacon_block_root,
                            prague.requests.requests_hash(),
                        ))?,
                    )
                }
            } else {
                (
                    EngineApiMessageVersion::V3,
                    serde_json::to_value((
                        payload,
                        cancun.versioned_hashes.clone(),
                        cancun.parent_beacon_block_root,
                    ))?,
                )
            }
        }
        ExecutionPayload::V2(payload) => {
            let input = ExecutionPayloadInputV2 {
                execution_payload: payload.payload_inner,
                withdrawals: Some(payload.withdrawals),
            };

            (EngineApiMessageVersion::V2, serde_json::to_value((input,))?)
        }
        ExecutionPayload::V1(payload) => {
            (EngineApiMessageVersion::V1, serde_json::to_value((payload,))?)
        }
    };

    Ok((version, params))
}

/// Calls the correct `engine_newPayload` method depending on the given [`ExecutionPayload`] and its
/// versioned variant. Returns the [`EngineApiMessageVersion`] depending on the payload's version.
///
/// # Panics
/// If the given payload is a V3 payload, but a parent beacon block root is provided as `None`.
pub(crate) async fn call_new_payload<N: Network, P: Provider<N>>(
    provider: P,
    version: EngineApiMessageVersion,
    params: serde_json::Value,
) -> TransportResult<()> {
    let method = version.method_name();
    tracing::info!("Calling engine API method: {}", method);

    let mut status: PayloadStatus = provider.client().request(method, &params).await?;

    while !status.is_valid() {
        if status.is_invalid() {
            error!(?status, ?params, "Invalid {method}",);
            panic!("Invalid {method}: {status:?}");
        }
        if status.is_syncing() {
            return Err(alloy_json_rpc::RpcError::UnsupportedFeature(
                "invalid range: no canonical state found for parent of requested block",
            ))
        }
        status = provider.client().request(method, &params).await?;
    }
    Ok(())
}

/// Calls the correct `engine_forkchoiceUpdated` method depending on the given
/// `EngineApiMessageVersion`, using the provided forkchoice state and payload attributes for the
/// actual engine api message call.
pub(crate) async fn call_forkchoice_updated<N, P: EngineApiValidWaitExt<N>>(
    provider: P,
    message_version: EngineApiMessageVersion,
    forkchoice_state: ForkchoiceState,
    payload_attributes: Option<PayloadAttributes>,
) -> TransportResult<ForkchoiceUpdated> {
    match message_version {
        EngineApiMessageVersion::V3 | EngineApiMessageVersion::V4 | EngineApiMessageVersion::V5 => {
            provider.fork_choice_updated_v3_wait(forkchoice_state, payload_attributes).await
        }
        EngineApiMessageVersion::V2 => {
            provider.fork_choice_updated_v2_wait(forkchoice_state, payload_attributes).await
        }
        EngineApiMessageVersion::V1 => {
            provider.fork_choice_updated_v1_wait(forkchoice_state, payload_attributes).await
        }
    }
}
