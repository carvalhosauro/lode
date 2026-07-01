//! Golden corpus evaluation — pure in-memory algorithm (RFC-0003 §12).
//!
//! Fixture loading (JSON/TOML from disk) lives in `lode-parse` tests, not here.

mod error;
mod miner;

pub use error::CorpusError;
pub use miner::{CorpusMiner, StubMiner};

use std::collections::{BTreeMap, BTreeSet};

use crate::mining::template_set_hash;

/// Per-format metadata carried in memory after fixtures are loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatSpec {
    pub id: String,
    pub pa_floor: f64,
}

/// Ground-truth label for one line (1-based line number).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineLabel {
    pub line: usize,
    pub template_gid: String,
    pub severity: Option<String>,
    pub severity_source: Option<String>,
}

/// One format's lines, labels, and template patterns — ready to evaluate.
#[derive(Debug, Clone)]
pub struct FormatInput {
    pub spec: FormatSpec,
    pub lines: Vec<String>,
    pub labels: BTreeMap<usize, LineLabel>,
    pub templates: BTreeMap<String, String>,
}

/// Full golden corpus in memory.
#[derive(Debug, Clone)]
pub struct CorpusInput {
    pub formats: Vec<FormatInput>,
}

/// One line assignment from a corpus run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAssignment {
    pub format_id: String,
    pub line: usize,
    pub template_gid: String,
    pub expected_pattern: String,
    pub mined_pattern: String,
}

/// Aggregated metrics from a full corpus pass.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusResult {
    pub assignments: Vec<LineAssignment>,
    pub template_set_hash: u64,
    pub per_format_pa: BTreeMap<String, f64>,
    pub global_pa: f64,
    pub per_format_template_count_delta: BTreeMap<String, i64>,
    pub global_template_count_delta: i64,
}

/// Run the miner over every format and compute PA / determinism artifacts.
///
/// # Errors
///
/// Returns [`CorpusError`] when a label references an unknown template id.
pub fn run_corpus(input: &CorpusInput, miner: &mut impl CorpusMiner) -> Result<CorpusResult, CorpusError> {
    let mut assignments = Vec::new();
    let mut mined_patterns = BTreeMap::<String, BTreeSet<String>>::new();
    let mut expected_patterns = BTreeMap::<String, BTreeSet<String>>::new();
    let mut per_format_correct = BTreeMap::<String, usize>::new();
    let mut per_format_total = BTreeMap::<String, usize>::new();

    for format in &input.formats {
        let format_id = &format.spec.id;
        let mut correct = 0usize;

        for (idx, raw) in format.lines.iter().enumerate() {
            let line_no = idx + 1;
            let label = format.labels.get(&line_no).ok_or_else(|| CorpusError::MissingLabel {
                format: format_id.clone(),
                line: line_no,
            })?;
            let expected = format
                .templates
                .get(&label.template_gid)
                .ok_or_else(|| CorpusError::UnknownTemplateGid {
                    format: format_id.clone(),
                    gid: label.template_gid.clone(),
                })?;

            let mined_pattern = miner.mine_line(raw);
            if patterns_match(expected, &mined_pattern) {
                correct += 1;
            }

            mined_patterns
                .entry(format_id.clone())
                .or_default()
                .insert(mined_pattern.clone());
            expected_patterns
                .entry(format_id.clone())
                .or_default()
                .insert(expected.clone());

            assignments.push(LineAssignment {
                format_id: format_id.clone(),
                line: line_no,
                template_gid: label.template_gid.clone(),
                expected_pattern: expected.clone(),
                mined_pattern,
            });
        }

        per_format_correct.insert(format_id.clone(), correct);
        per_format_total.insert(format_id.clone(), format.lines.len());
    }

    let mut all_mined: Vec<String> = mined_patterns
        .values()
        .flat_map(BTreeSet::iter)
        .cloned()
        .collect();
    all_mined.sort();
    all_mined.dedup();
    let pattern_refs: Vec<&str> = all_mined.iter().map(String::as_str).collect();
    let template_set_hash = template_set_hash(&pattern_refs);

    let mut per_format_pa = BTreeMap::new();
    let mut weighted_correct = 0usize;
    let mut weighted_total = 0usize;
    for format in &input.formats {
        let id = &format.spec.id;
        let correct = per_format_correct[id];
        let total = per_format_total[id];
        per_format_pa.insert(id.clone(), pa_ratio(correct, total));
        weighted_correct += correct;
        weighted_total += total;
    }

    let mut per_format_template_count_delta = BTreeMap::new();
    let mut mined_total = 0i64;
    let mut expected_total = 0i64;
    for format in &input.formats {
        let id = &format.spec.id;
        let mined_count = pattern_set_len_i64(mined_patterns.get(id));
        let expected_count = pattern_set_len_i64(expected_patterns.get(id));
        let delta = mined_count - expected_count;
        per_format_template_count_delta.insert(id.clone(), delta);
        mined_total += mined_count;
        expected_total += expected_count;
    }

    Ok(CorpusResult {
        assignments,
        template_set_hash,
        per_format_pa,
        global_pa: pa_ratio(weighted_correct, weighted_total),
        per_format_template_count_delta,
        global_template_count_delta: mined_total - expected_total,
    })
}

/// Whether a mined pattern matches ground truth (v1: exact string equality).
#[must_use]
pub fn patterns_match(expected: &str, mined: &str) -> bool {
    expected == mined
}

fn pattern_set_len_i64(set: Option<&BTreeSet<String>>) -> i64 {
    #[allow(clippy::cast_possible_wrap)] // corpus template counts are tiny
    {
        set.map_or(0, BTreeSet::len) as i64
    }
}

#[must_use]
#[allow(clippy::cast_precision_loss)] // corpus line counts are far below f64 mantissa limits
pub fn pa_ratio(correct: usize, total: usize) -> f64 {
    if total == 0 {
        return 1.0;
    }
    correct as f64 / total as f64
}

/// Run the corpus twice and check bit-identical outputs (RFC-0003 determinism).
///
/// # Errors
///
/// Returns [`CorpusError::DeterminismMismatch`] when the two runs diverge.
pub fn assert_deterministic<F, M>(input: &CorpusInput, mut miner_factory: F) -> Result<(), CorpusError>
where
    F: FnMut() -> M,
    M: CorpusMiner,
{
    let mut miner_a = miner_factory();
    let run_a = run_corpus(input, &mut miner_a)?;
    let mut miner_b = miner_factory();
    let run_b = run_corpus(input, &mut miner_b)?;

    if run_a.template_set_hash != run_b.template_set_hash {
        return Err(CorpusError::DeterminismMismatch {
            detail: format!(
                "template_set_hash mismatch: {:#x} vs {:#x}",
                run_a.template_set_hash, run_b.template_set_hash
            ),
        });
    }
    if run_a.assignments != run_b.assignments {
        return Err(CorpusError::DeterminismMismatch {
            detail: "line assignments differ between runs".to_string(),
        });
    }
    Ok(())
}

impl CorpusResult {
    /// Recompute `template_set_hash` from assignments (cross-check).
    #[must_use]
    pub fn recompute_template_set_hash(&self) -> u64 {
        let mut patterns: Vec<String> = self
            .assignments
            .iter()
            .map(|a| a.mined_pattern.clone())
            .collect();
        patterns.sort();
        patterns.dedup();
        let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
        template_set_hash(&refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPatternMiner;

    impl CorpusMiner for FixedPatternMiner {
        fn mine_line(&mut self, _raw: &str) -> String {
            "GET <PATH>".to_string()
        }
    }

    fn sample_input() -> CorpusInput {
        CorpusInput {
            formats: vec![FormatInput {
                spec: FormatSpec {
                    id: "sample".to_string(),
                    pa_floor: 0.9,
                },
                lines: vec!["x".to_string(), "y".to_string()],
                labels: BTreeMap::from([
                    (
                        1,
                        LineLabel {
                            line: 1,
                            template_gid: "t1".to_string(),
                            severity: None,
                            severity_source: None,
                        },
                    ),
                    (
                        2,
                        LineLabel {
                            line: 2,
                            template_gid: "t1".to_string(),
                            severity: None,
                            severity_source: None,
                        },
                    ),
                ]),
                templates: BTreeMap::from([("t1".to_string(), "GET <PATH>".to_string())]),
            }],
        }
    }

    #[test]
    fn patterns_match_is_exact() {
        assert!(patterns_match("GET <PATH>", "GET <PATH>"));
        assert!(!patterns_match("GET <PATH>", "POST <PATH>"));
    }

    #[test]
    fn pa_ratio_handles_empty() {
        assert!((pa_ratio(0, 0) - 1.0).abs() < f64::EPSILON);
        assert!((pa_ratio(9, 10) - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn run_corpus_computes_pa() {
        let input = sample_input();
        let mut miner = FixedPatternMiner;
        let result = run_corpus(&input, &mut miner).expect("run");
        assert!((result.global_pa - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.assignments.len(), 2);
    }

    #[test]
    fn assert_deterministic_with_fixed_miner() {
        let input = sample_input();
        assert_deterministic(&input, || FixedPatternMiner).expect("deterministic");
    }

    #[test]
    fn template_set_hash_recomputes() {
        let input = sample_input();
        let mut miner = FixedPatternMiner;
        let result = run_corpus(&input, &mut miner).expect("run");
        assert_eq!(
            result.template_set_hash,
            result.recompute_template_set_hash()
        );
    }
}
