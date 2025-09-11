use jsonrpsee_core::server::RpcModule;
use reth::rpc::api::IntoEngineApiRpcModule;
use reth_engine_primitives::BeaconConsensusEngineHandle;
use std::sync::Arc;

#[cfg(feature = "bench-test")]
use alloy_rpc_types::engine::{ForkchoiceState, PayloadStatusEnum};
#[cfg(feature = "bench-test")]
use jsonrpsee_types::error::ErrorObjectOwned;
#[cfg(feature = "bench-test")]
use reth_payload_primitives::EngineApiMessageVersion;
#[cfg(feature = "bench-test")]
use reth_node_ethereum::engine::EthPayloadAttributes;
use crate::consensus::ParliaConsensus;

pub mod builder;
pub mod payload;
pub mod validator;

#[derive(Clone)]
pub struct BscEngineApi<Provider> {
    /// Handle to the beacon consensus engine
    #[allow(dead_code)]
    engine_handle:
        Arc<BeaconConsensusEngineHandle<crate::node::engine_api::payload::BscPayloadTypes>>,
    /// The consensus implementation for canonical head checks
    #[allow(dead_code)] // Used in #[cfg(feature = "bench-test")] code
    consensus: Arc<ParliaConsensus<Provider>>,
}


impl<Provider> BscEngineApi<Provider> {
    /// Create a new BSC Engine API instance
    pub fn new(
        engine_handle: Arc<
            BeaconConsensusEngineHandle<crate::node::engine_api::payload::BscPayloadTypes>,
        >,
        consensus: Arc<ParliaConsensus<Provider>>,
    ) -> Self 
    where
        Provider: Send + Sync + 'static,
    {
        Self { 
            engine_handle,
            consensus,
        }
    }
}

impl<Provider> IntoEngineApiRpcModule for BscEngineApi<Provider>
where
    Provider: Send + Sync + 'static,
{
    fn into_rpc_module(self) -> RpcModule<()> {
        #[cfg(feature = "bench-test")]
        let mut module = RpcModule::new(());
        #[cfg(not(feature = "bench-test"))]
        let module = RpcModule::new(());

        // Register the fork choice update v1 method only when bench-test feature is enabled
        #[cfg(feature = "bench-test")]
        {
            module
                .register_async_method("engine_forkchoiceUpdatedV1", move |params, _, _| {
                    let engine_handle = self.engine_handle.clone();
                    let consensus = self.consensus.clone();

                    async move {
                        // Parse the parameters - ForkchoiceState and optional PayloadAttributes
                        let (forkchoice_state, payload_attrs): (
                            ForkchoiceState,
                            Option<EthPayloadAttributes>,
                        ) = params.parse().map_err(|e| {
                            ErrorObjectOwned::owned(-32602, format!("Parse error: {}", e), None::<()>)
                        })?;

                        // Get block number for the head block hash
                        let head_block_number = match consensus.provider.best_block_number() {
                            Ok(number) => number,
                            Err(_) => return Err(ErrorObjectOwned::owned(
                                -32603,
                                "Engine error: failed to get block number",
                                None::<()>,
                            )),
                        };

                        let (head_block_hash, current_hash) = match consensus.canonical_head(forkchoice_state.head_block_hash, head_block_number) {
                            Ok(hash) => hash,
                            Err(_) => return Err(ErrorObjectOwned::owned(
                                -32603,
                                "Engine error: links to previously rejected block",
                                None::<()>,
                            )),
                        };

                        let engine = engine_handle.clone();
                        // Call the engine service
                        match engine
                            .fork_choice_updated(
                                forkchoice_state,
                                payload_attrs,
                                EngineApiMessageVersion::V1,
                            )
                            .await
                        {
                            Ok(response) => match response.payload_status.status {
                                PayloadStatusEnum::Valid => Ok(response),
                                PayloadStatusEnum::Invalid { validation_error } => {
                                    Err(ErrorObjectOwned::owned(
                                        -32603,
                                        format!("Engine error: {}", validation_error),
                                        None::<()>,
                                    ))
                                }
                                _ => Err(ErrorObjectOwned::owned(
                                    -32603,
                                    format!("Engine status error: {}", response.payload_status.status),
                                    None::<()>,
                                )),
                            },
                            Err(err) => Err(ErrorObjectOwned::owned(
                                -32603,
                                format!("Engine fork_choice_updated error: {}", err),
                                None::<()>,
                            )),
                        }
                    }
                })
                .expect("Failed to register engine_forkchoiceUpdatedV1");
        }

        module
    }
}
