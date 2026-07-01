//! Load `fixtures/corpus/` into [`lode_core::CorpusInput`] (integration tests only).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use lode_core::{CorpusInput, FormatInput, FormatSpec, LineLabel};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ManifestFile {
    format: Vec<ManifestFormat>,
}

#[derive(Debug, Deserialize)]
struct ManifestFormat {
    id: String,
    path: String,
    lines: usize,
    #[serde(rename = "source_type")]
    _source_type: String,
    pa_floor: f64,
}

#[derive(Debug, Deserialize)]
struct LabelRow {
    line: usize,
    template_gid: String,
    severity: Option<String>,
    severity_source: Option<String>,
}

/// Load the golden corpus from `root` (`manifest.toml` + per-format subdirs).
///
/// # Panics
///
/// Panics if fixture files are missing, malformed, or internally inconsistent.
pub(crate) fn load_corpus(root: impl AsRef<Path>) -> CorpusInput {
    load_corpus_result(root).expect("golden corpus fixtures must load")
}

/// Fallible loader for explicit error messages in tests.
///
/// # Errors
///
/// Returns a formatted error string when fixture files are missing, malformed, or inconsistent.
pub(crate) fn load_corpus_result(root: impl AsRef<Path>) -> Result<CorpusInput, String> {
    let root = root.as_ref();
    let manifest_path = root.join("manifest.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: ManifestFile = toml::from_str(&manifest_text)
        .map_err(|e| format!("parse {}: {e}", manifest_path.display()))?;

    let mut formats = Vec::with_capacity(manifest.format.len());
    for entry in manifest.format {
        formats.push(load_format(root, &entry)?);
    }
    Ok(CorpusInput { formats })
}

fn load_format(root: &Path, entry: &ManifestFormat) -> Result<FormatInput, String> {
    let dir = root.join(&entry.path);
    let input_path = dir.join("input.log");
    let labels_path = dir.join("labels.jsonl");
    let templates_path = dir.join("templates.json");

    let input_text = fs::read_to_string(&input_path)
        .map_err(|e| format!("read {}: {e}", input_path.display()))?;
    let lines: Vec<String> = input_text.lines().map(str::to_string).collect();

    let labels_text = fs::read_to_string(&labels_path)
        .map_err(|e| format!("read {}: {e}", labels_path.display()))?;
    let mut labels = BTreeMap::new();
    for (idx, row) in labels_text.lines().enumerate() {
        if row.trim().is_empty() {
            continue;
        }
        let label: LabelRow = serde_json::from_str(row)
            .map_err(|e| format!("parse {} line {}: {e}", labels_path.display(), idx + 1))?;
        labels.insert(
            label.line,
            LineLabel {
                line: label.line,
                template_gid: label.template_gid,
                severity: label.severity,
                severity_source: label.severity_source,
            },
        );
    }

    if lines.len() != labels.len() {
        return Err(format!(
            "format {}: input.log has {} lines, labels.jsonl has {}",
            entry.id,
            lines.len(),
            labels.len()
        ));
    }
    for line_no in 1..=lines.len() {
        if !labels.contains_key(&line_no) {
            return Err(format!(
                "format {}: missing label for line {line_no}",
                entry.id
            ));
        }
    }

    let templates_text = fs::read_to_string(&templates_path)
        .map_err(|e| format!("read {}: {e}", templates_path.display()))?;
    let templates: BTreeMap<String, String> = serde_json::from_str(&templates_text)
        .map_err(|e| format!("parse {}: {e}", templates_path.display()))?;

    if entry.lines != lines.len() {
        return Err(format!(
            "format {}: manifest lines={} but input.log has {}",
            entry.id,
            entry.lines,
            lines.len()
        ));
    }

    Ok(FormatInput {
        spec: FormatSpec {
            id: entry.id.clone(),
            pa_floor: entry.pa_floor,
        },
        lines,
        labels,
        templates,
    })
}

/// Workspace-relative corpus root from `CARGO_MANIFEST_DIR` of `lode-parse`.
#[must_use]
pub(crate) fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus")
}
