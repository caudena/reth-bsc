//! Contains various benchmark output formats, either for logging or for
//! serialization to / from files.

use reth_primitives_traits::constants::GIGAGAS;
use serde::{ser::SerializeStruct, Serialize};
use std::time::Duration;

/// This is the suffix for gas output csv files.
pub(crate) const GAS_OUTPUT_SUFFIX: &str = "total_gas.csv";



/// This is the suffix for forkchoice output csv files.
pub(crate) const FORKCHOICE_OUTPUT_SUFFIX: &str = "forkchoice_latency.csv";



/// This represents the results of a single `forkchoiceUpdated` call in the benchmark, containing the gas
/// used and the `forkchoiceUpdated` latency.
#[derive(Debug)]
pub(crate) struct ForkchoiceResult {
    /// The gas used in the `forkchoiceUpdated` call.
    pub(crate) gas_used: u64,
    /// The latency of the `forkchoiceUpdated` call.
    pub(crate) latency: Duration,
}

impl ForkchoiceResult {
    /// Returns the gas per second processed in the `forkchoiceUpdated` call.
    pub(crate) fn gas_per_second(&self) -> f64 {
        self.gas_used as f64 / self.latency.as_secs_f64()
    }
}

impl std::fmt::Display for ForkchoiceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Forkchoice update processed at {:.4} Ggas/s, used {} total gas. Latency: {:?}",
            self.gas_per_second() / GIGAGAS as f64,
            self.gas_used,
            self.latency
        )
    }
}

impl Serialize for ForkchoiceResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // convert the time to microseconds
        let time = self.latency.as_micros();
        let mut state = serializer.serialize_struct("ForkchoiceResult", 2)?;
        state.serialize_field("gas_used", &self.gas_used)?;
        state.serialize_field("latency", &time)?;
        state.end()
    }
}



/// This represents a row of total gas data in the benchmark.
#[derive(Debug)]
pub(crate) struct TotalGasRow {
    /// The block number of the block being processed.
    pub(crate) block_number: u64,
    /// The total gas used in the block.
    pub(crate) gas_used: u64,
    /// Time since the start of the benchmark.
    pub(crate) time: Duration,
}

/// This represents the aggregated output, meant to show gas per second metrics, of a benchmark run.
#[derive(Debug)]
pub(crate) struct TotalGasOutput {
    /// The total gas used in the benchmark.
    pub(crate) total_gas_used: u64,
    /// The total duration of the benchmark.
    pub(crate) total_duration: Duration,
    /// The total gas used per second.
    pub(crate) total_gas_per_second: f64,
    /// The number of blocks processed.
    pub(crate) blocks_processed: u64,
}

impl TotalGasOutput {
    /// Create a new [`TotalGasOutput`] from a list of [`TotalGasRow`].
    pub(crate) fn new(rows: Vec<TotalGasRow>) -> Self {
        // the duration is obtained from the last row
        let total_duration =
            rows.last().map(|row| row.time).expect("the row has at least one element");
        let blocks_processed = rows.len() as u64;
        let total_gas_used: u64 = rows.into_iter().map(|row| row.gas_used).sum();
        let total_gas_per_second = total_gas_used as f64 / total_duration.as_secs_f64();

        Self { total_gas_used, total_duration, total_gas_per_second, blocks_processed }
    }

    /// Return the total gigagas per second.
    pub(crate) fn total_gigagas_per_second(&self) -> f64 {
        self.total_gas_per_second / GIGAGAS as f64
    }
}

/// This serializes the `time` field of the [`TotalGasRow`] to microseconds.
///
/// This is essentially just for the csv writer, which would have headers
impl Serialize for TotalGasRow {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // convert the time to microseconds
        let time = self.time.as_micros();
        let mut state = serializer.serialize_struct("TotalGasRow", 3)?;
        state.serialize_field("block_number", &self.block_number)?;
        state.serialize_field("gas_used", &self.gas_used)?;
        state.serialize_field("time", &time)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::Writer;
    use std::io::BufRead;

    #[test]
    fn test_write_total_gas_row_csv() {
        let row = TotalGasRow { block_number: 1, gas_used: 1_000, time: Duration::from_secs(1) };

        let mut writer = Writer::from_writer(vec![]);
        writer.serialize(row).unwrap();
        let result = writer.into_inner().unwrap();

        // parse into Lines
        let mut result = result.as_slice().lines();

        // assert header
        let expected_first_line = "block_number,gas_used,time";
        let first_line = result.next().unwrap().unwrap();
        assert_eq!(first_line, expected_first_line);

        let expected_second_line = "1,1000,1000000";
        let second_line = result.next().unwrap().unwrap();
        assert_eq!(second_line, expected_second_line);
    }
}
