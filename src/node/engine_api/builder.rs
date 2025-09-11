use super::BscEngineApi;
use reth::{
    api::{AddOnsContext, FullNodeComponents, NodeTypes},
    builder::rpc::EngineApiBuilder,
};
use std::sync::Arc;

/// Builder for [`BscEngineApi`] implementation.
#[derive(Debug, Default)]
pub struct BscEngineApiBuilder;

impl<N> EngineApiBuilder<N> for BscEngineApiBuilder
where
    N: FullNodeComponents,
    N::Types: NodeTypes<Payload = crate::node::engine_api::payload::BscPayloadTypes>,
{
    type EngineApi = BscEngineApi<N::Provider>;

    async fn build_engine_api(self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::EngineApi> {
        // Get the engine handle from the context
        let engine_handle = ctx.beacon_engine_handle.clone();
        
        // Get consensus for canonical head checks
        let provider = ctx.node.provider().clone();
        let parlia_consensus = std::sync::Arc::new(crate::consensus::ParliaConsensus { provider });
        Ok(BscEngineApi::new(Arc::new(engine_handle), parlia_consensus))
    }
}
