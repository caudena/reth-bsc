# reth-bench-bsc

BSC-compatible benchmarking tool for Ethereum nodes, based on reth-bench.

## Features

- **BSC Network Support**: Automatically detects BSC Mainnet (Chain ID: 56) and BSC Testnet (Chain ID: 97)
- **Independent Operation**: Works standalone without affecting the parent reth-bsc repository
- **Compatible APIs**: Same interface as original reth-bench but with BSC-specific optimizations
- **Benchmark Mode**: 
  - `forkchoice-only`: Only calls subsequent `forkchoiceUpdated` calls

## Building

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Show help
./target/debug/reth-bench-bsc --help

# Benchmark with forkchoice-only mode
./target/debug/reth-bench-bsc forkchoice-only \
  --rpc-url <RPC_URL> \
  --engine-rpc-url <ENGINE_RPC_URL> \
  --jwt-secret <JWT_SECRET_PATH>
```

### BSC-Specific Examples

```bash
# Benchmark against BSC Mainnet
./target/debug/reth-bench-bsc forkchoice-only \
  --rpc-url https://bsc-dataseed.bnbchain.org \
  --engine-rpc-url http://localhost:8551 \
  --jwt-secret ./jwt.hex

# Benchmark against BSC Testnet
./target/debug/reth-bench-bsc forkchoice-only \
  --rpc-url https://data-seed-prebsc-1-s1.bnbchain.org:8545 \
  --engine-rpc-url http://localhost:8551 \
  --jwt-secret ./jwt.hex
```

### Options

- `--rpc-url`: RPC endpoint to fetch block data from
- `--engine-rpc-url`: Engine API endpoint for sending payloads
- `--jwt-secret`: Path to JWT secret file for engine API authentication
- `--from`: Starting block number for benchmark range
- `--to`: Ending block number for benchmark range
- `--output`: Output directory for benchmark results

## BSC Network Detection

The tool automatically detects BSC networks by chain ID:
- **BSC Mainnet**: Chain ID 56
- **BSC Testnet**: Chain ID 97

When a BSC network is detected, the tool will log:
```
Detected BSC Mainnet (Chain ID: 56)
```

## Output

Benchmark results are saved as CSV files in the specified output directory:
- Gas usage statistics
- Timing measurements  
- Payload processing metrics

## Dependencies

This tool uses the BSC fork of reth (https://github.com/bnb-chain/reth) to ensure compatibility with BSC-specific features and hardforks.

## License

MIT OR Apache-2.0
