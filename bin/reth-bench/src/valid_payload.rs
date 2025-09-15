//! This is an extension trait for any provider that implements the engine API, to wait for a VALID
//! response. This is useful for benchmarking, as it allows us to wait for a payload to be valid
//! before sending additional calls.

use alloy_provider::{ext::EngineApi, Network, Provider};
use alloy_rpc_types_engine::{
    ForkchoiceState, ForkchoiceUpdated,
    PayloadAttributes,
};
use alloy_transport::TransportResult;
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
                tracing::debug!("Node is syncing, waiting 200ms before retry...");
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
                tracing::debug!("Node is syncing, waiting 200ms before retry...");
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
                tracing::debug!("Node is syncing, waiting 200ms before retry...");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // Continue the loop to retry
            }
            status =
                self.fork_choice_updated_v3(fork_choice_state, payload_attributes.clone()).await?;
        }

        Ok(status)
    }
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
