//! [`DrainMiner`] — tokenize, mask, and drain wiring for corpus mining.

use lode_core::{CorpusMiner, DrainState, MiningParams};

use crate::mask::tokenize_and_mask;

/// Per-format overrides of the RFC-0003 mining tunables, applied by
/// [`DrainMiner::begin_format`].
///
/// Combined-log formats (nginx today; others such as Apache later) need parse-tree
/// depth `d > 4` so the method/version land in the routing prefix (RFC-0003 §11,
/// §13). This is a stopgap **data table**, not config-as-code: RFC-0016 will grow a
/// declarative per-stream `mining` surface (issue #8), after which these overrides
/// move to configuration and a new combined-log format no longer needs a Rust edit.
const FORMAT_DEPTH_OVERRIDES: &[(&str, u8)] = &[("nginx-access", 5)];

/// Resolve the mining params for `format_id`: defaults plus any registered override.
fn params_for(format_id: &str) -> MiningParams {
    let mut params = MiningParams::default();
    if let Some(&(_, depth)) = FORMAT_DEPTH_OVERRIDES
        .iter()
        .find(|&&(id, _)| id == format_id)
    {
        params.depth = depth;
    }
    params
}

/// Mines log lines via tokenization, masking, and drain-family template clustering.
pub struct DrainMiner {
    state: DrainState,
    /// Whether [`begin_format`](CorpusMiner::begin_format) has run. Mining before it
    /// silently uses default depth `d = 4`, collapsing combined-log PA to ~2%
    /// (docs/phase-1-spec.md); the `debug_assert!` in `mine_line` makes that loud.
    began: bool,
}

impl DrainMiner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: DrainState::new(MiningParams::default()),
            began: false,
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
        self.state.reset(params_for(format_id));
        self.began = true;
    }

    fn mine_line(&mut self, raw: &str) -> String {
        debug_assert!(
            self.began,
            "mine_line called before begin_format: format-specific mining params were \
             not applied, so mining runs at default depth and combined-log PA collapses"
        );
        match tokenize_and_mask(raw) {
            Ok(masked) => self.state.process(&masked).pattern,
            Err(error) => {
                // The golden corpus has no malformed lines, so a parse error means the
                // fixture or tokenizer regressed. Fail loud in dev; the empty string is
                // kept for release until T3.2 adds real degraded handling — otherwise it
                // enters `run_corpus`'s pattern set as a silent phantom template.
                debug_assert!(
                    false,
                    "corpus line failed to parse ({error:?}); line: {raw:?}"
                );
                String::new()
            }
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

    #[test]
    fn nginx_gets_depth_override_others_default() {
        assert_eq!(params_for("nginx-access").depth, 5);
        assert_eq!(
            params_for("json-lines").depth,
            MiningParams::default().depth
        );
        assert_eq!(
            params_for("syslog-rfc5424").depth,
            MiningParams::default().depth
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "begin_format")]
    fn mine_line_before_begin_format_is_loud_in_debug() {
        let mut m = DrainMiner::new();
        let _ = m.mine_line("anything");
    }
}
