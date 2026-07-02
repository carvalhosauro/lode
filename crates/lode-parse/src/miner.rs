//! [`DrainMiner`] — tokenize, mask, and drain wiring for corpus mining.

use lode_core::{CorpusMiner, DrainState, MiningParams};

use crate::mask::tokenize_and_mask;

/// Mines log lines via tokenization, masking, and drain-family template clustering.
pub struct DrainMiner {
    state: DrainState,
}

impl DrainMiner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: DrainState::new(MiningParams::default()),
        }
    }
}

impl Default for DrainMiner {
    fn default() -> Self {
        Self::new()
    }
}

impl CorpusMiner for DrainMiner {
    fn begin_format(&mut self, format_id: &str) {
        let params = if format_id == "nginx-access" {
            MiningParams {
                depth: 5,
                ..MiningParams::default()
            }
        } else {
            MiningParams::default()
        };
        self.state.reset(params);
    }

    fn mine_line(&mut self, raw: &str) -> String {
        match tokenize_and_mask(raw) {
            Ok(masked) => self.state.process(&masked).pattern,
            Err(_) => String::new(), // corpus has no bad lines; T3.2 adds degraded handling
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mines_nginx_line() {
        let mut m = DrainMiner::new();
        m.begin_format("nginx-access");
        let pat = m.mine_line(
            r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#,
        );
        assert!(pat.contains("GET <PATH>"));
    }
}
