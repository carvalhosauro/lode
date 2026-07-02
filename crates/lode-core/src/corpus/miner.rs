//! [`CorpusMiner`] trait and placeholder miners.

/// Mines a single raw log line into its template pattern string.
pub trait CorpusMiner {
    fn mine_line(&mut self, raw: &str) -> String;
}

/// Placeholder until [`DrainMiner`](crate::mining) lands in T1. Always returns empty.
///
/// Not used in quality gates — only for wiring smoke tests outside the corpus suite.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubMiner;

impl CorpusMiner for StubMiner {
    fn mine_line(&mut self, _raw: &str) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_miner_always_returns_empty() {
        let mut miner = StubMiner;
        assert_eq!(miner.mine_line("GET /api"), "");
        assert_eq!(miner.mine_line(""), "");
    }
}
