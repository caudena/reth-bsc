# Reth @ BSC

A BSC-compatible Reth client implementation. This project is **not** a fork of Reth, but rather an extension that leverages Reth's powerful `NodeBuilder` API to provide BSC compatibility.

## About

This project aims to bring Reth's high-performance Ethereum client capabilities to the BSC network. By utilizing Reth's modular architecture and NodeBuilder API, we're building a BSC-compatible client that maintains compatibility with Reth's ecosystem while adding BSC-specific features.

## Current Status

- Historical Sync ✅
- BSC Pectra Support ✅
- Live Sync ✅
- Run as validator ❌ (soon)

### Sync Status (as of September 1st, 2025)

- **BSC Mainnet**: Synced to the tip ✅ (12T disk required)
- **BSC Testnet**: Synced to the tip ✅ (780GB disk usage)

## Getting Started

Refer to the [Reth documentation](https://reth.rs/) for general guidance on running a node. Note that some BSC-specific configurations may be required.

### EVN Support

This client implements the BSC upgrade-status handshake extension. When EVN is enabled, the node requests peers to disable transaction broadcast towards it, mirroring the Enhanced Validator Network behavior used by validator/sentry nodes.

- Enable via CLI: `--evn.enabled`
- Or via env var: `BSC_EVN_ENABLED=true`
- EVN activates only after the node is synced (based on head timestamp lag). Override lag threshold via `BSC_EVN_SYNC_LAG_SECS` (default 30s). Existing peers are refreshed once EVN is armed.

Note: This currently affects the outgoing handshake signaling. Further EVN behaviors (e.g., peer whitelists, conditional broadcast policies) can be added incrementally.

## Contributing

We welcome community contributions! Whether you're interested in helping with historical sync implementation, BSC Pectra support, or live sync functionality, your help is valuable. Please feel free to open issues or submit pull requests. You can reach out to me on [Telegram](https://t.me/loocapro).

## Disclaimer

This project is experimental and under active development. Use at your own risk.

## Credits

This project is inspired by and builds upon the work of:

- [BNB Chain Reth](https://github.com/bnb-chain/reth) - The original BSC implementation of Reth
- The Reth team, especially [@mattsse](https://github.com/mattsse) for their invaluable contributions to the Reth ecosystem

## Acknowledgements from BNBChain team

This project based on the excellent community versions as foundation, We extend our sincere appreciation to the teams below:
- [Reth-bsc](https://github.com/loocapro/reth-bsc) - The BSC Reth implementation from community
- [Reth](https://github.com/paradigmxyz/reth) - The reth project
- Especially thanks to:
  - [@loocapro](https://github.com/loocapro)
  - [@mattsse](https://github.com/mattsse)
  - [@klkvr](https://github.com/klkvr)
  - All contributors on reth and reth-bsc
