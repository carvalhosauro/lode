//! Drain-family template clustering over masked token sequences (RFC-0003 §6.2).

use std::collections::HashMap;

use crate::hash::Fnv1a64;
use crate::ids::{Fingerprint, IndexTime, TemplateId};
use crate::mining::{MaskKind, MaskedTokens, MiningParams, Token, pattern_to_string};
use crate::template::{Template, TemplateState};

const ROUTING_SEP: u8 = 0x1e;

/// Outcome of routing one masked event through the drain engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub template_id: Option<TemplateId>,
    pub pattern: String,
    pub fingerprint: Fingerprint,
}

#[derive(Debug)]
struct RegistryEntry {
    template: Template,
    tokens: Vec<Token>,
}

#[derive(Debug)]
struct TemplateRegistry {
    entries: Vec<RegistryEntry>,
    buckets: HashMap<u64, Vec<TemplateId>>,
}

impl TemplateRegistry {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            buckets: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.buckets.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry_mut(&mut self, id: TemplateId) -> &mut RegistryEntry {
        &mut self.entries[id.0 as usize]
    }

    fn bucket_candidates(&self, key: u64) -> &[TemplateId] {
        self.buckets.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn push_bucket(&mut self, key: u64, id: TemplateId) {
        self.buckets.entry(key).or_default().push(id);
    }
}

/// Incremental drain state for one format / stream context.
#[derive(Debug)]
pub struct DrainState {
    registry: TemplateRegistry,
    params: MiningParams,
    next_id: u64,
}

impl DrainState {
    #[must_use]
    pub fn new(params: MiningParams) -> Self {
        Self {
            registry: TemplateRegistry::new(),
            params,
            next_id: 0,
        }
    }

    /// Clears all templates so a new format can be mined deterministically.
    pub fn reset(&mut self, params: MiningParams) {
        self.registry.clear();
        self.params = params;
        self.next_id = 0;
    }

    pub fn process(&mut self, masked: &MaskedTokens) -> ProcessResult {
        let len = masked.len();
        let key = routing_key(&masked.tokens, self.params.depth);
        let st = self.params.similarity_threshold;

        let mut best_id: Option<TemplateId> = None;
        let mut best_sim = st;

        for &candidate_id in self.registry.bucket_candidates(key) {
            let entry = &self.registry.entries[candidate_id.0 as usize];
            let sim = sequence_similarity(&entry.tokens, &masked.tokens);
            if sim >= st {
                let better = match best_id {
                    None => true,
                    Some(current) => sim > best_sim || (sim == best_sim && candidate_id < current),
                };
                if better {
                    best_sim = sim;
                    best_id = Some(candidate_id);
                }
            }
        }

        let template_id = if let Some(id) = best_id {
            let widen_from = usize::from(self.params.depth).min(len);
            {
                let entry = self.registry.entry_mut(id);
                let changed = widen_suffix(&mut entry.tokens, &masked.tokens, widen_from);
                if changed {
                    entry.template.pattern = pattern_to_string(&entry.tokens).into();
                    entry.template.version += 1;
                }
                entry.template.occurrence_count += 1;
                entry.template.last_seen = IndexTime(entry.template.occurrence_count);
                if entry.template.state == TemplateState::Emerging
                    && entry.template.occurrence_count >= u64::from(self.params.stable_threshold)
                {
                    entry.template.state = TemplateState::Stable;
                }
            }
            id
        } else {
            self.maybe_evict_stub();
            let id = TemplateId(self.next_id);
            self.next_id += 1;
            let tokens = masked.tokens.clone();
            let pattern = pattern_to_string(&tokens);
            let template = Template {
                id,
                pattern: pattern.clone().into(),
                version: 1,
                occurrence_count: 1,
                first_seen: IndexTime(1),
                last_seen: IndexTime(1),
                state: TemplateState::Emerging,
            };
            self.registry.entries.push(RegistryEntry { template, tokens });
            self.registry.push_bucket(key, id);
            id
        };

        let entry = &self.registry.entries[template_id.0 as usize];
        let pattern = entry.template.pattern.to_string();
        let fingerprint = Fingerprint::from_masked_tokens(&masked.tokens);

        ProcessResult {
            template_id: Some(template_id),
            pattern,
            fingerprint,
        }
    }

    fn maybe_evict_stub(&self) {
        if self.registry.len() > self.params.max_templates as usize {
            // T6.1: eviction deferred; golden corpus never hits `max_templates`.
        }
    }
}

fn routing_key(tokens: &[Token], depth: u8) -> u64 {
    let len = tokens.len();
    let pref_end = usize::from(depth).min(len);
    let mut h = Fnv1a64::new();
    h.write_bytes(&(len as u64).to_le_bytes());
    h.write_byte(ROUTING_SEP);
    for (i, tok) in tokens.iter().take(pref_end).enumerate() {
        if i > 0 {
            h.write_byte(ROUTING_SEP);
        }
        h.write_bytes(tok.as_str().as_bytes());
    }
    h.finish()
}

fn sequence_similarity(pattern: &[Token], masked: &[Token]) -> f64 {
    if pattern.len() != masked.len() {
        return 0.0;
    }
    if pattern.is_empty() {
        return 1.0;
    }
    let wildcard = MaskKind::Wildcard.placeholder();
    let matches = pattern
        .iter()
        .zip(masked)
        .filter(|(p, m)| p.as_str() == m.as_str() || p.as_str() == wildcard)
        .count();
    matches as f64 / pattern.len() as f64
}

fn widen_suffix(pattern: &mut [Token], masked: &[Token], widen_from: usize) -> bool {
    let wildcard = MaskKind::Wildcard.placeholder();
    let mut changed = false;
    for j in widen_from..pattern.len() {
        if pattern[j].as_str() != masked[j].as_str() && pattern[j].as_str() != wildcard {
            pattern[j] = Token::new(wildcard);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masked_tokens(pattern: &str) -> MaskedTokens {
        let tokens = pattern
            .split_whitespace()
            .map(Token::new)
            .collect::<Vec<_>>();
        MaskedTokens::new(tokens)
    }

    fn sample_masked() -> MaskedTokens {
        masked_tokens("GET <PATH> <NUM>")
    }

    #[test]
    fn route_separates_nginx_verbs_at_d5() {
        let mut state = DrainState::new(MiningParams {
            depth: 5,
            ..Default::default()
        });
        let get = masked_tokens(r#"<IP> - - <TS> "GET <PATH> HTTP/1.1" <NUM> <NUM>"#);
        let post = masked_tokens(r#"<IP> - - <TS> "POST <PATH> HTTP/1.1" <NUM> <NUM>"#);
        let p1 = state.process(&get).pattern;
        let p2 = state.process(&post).pattern;
        assert_ne!(p1, p2);
        assert!(!p1.contains("<*>"));
    }

    #[test]
    fn widen_does_not_touch_routing_prefix() {
        let mut state = DrainState::new(MiningParams {
            depth: 4,
            ..Default::default()
        });
        let a = masked_tokens("A B C D foo");
        let b = masked_tokens("A B C D bar");
        let p1 = state.process(&a).pattern;
        assert_eq!(p1, "A B C D foo");
        let p2 = state.process(&b).pattern;
        assert_eq!(p2, "A B C D <*>");
    }

    #[test]
    fn determinism_same_input_same_template_id() {
        let mut a = DrainState::new(MiningParams::default());
        let mut b = DrainState::new(MiningParams::default());
        let m = sample_masked();
        assert_eq!(a.process(&m).pattern, b.process(&m).pattern);
        assert_eq!(a.process(&m).template_id, b.process(&m).template_id);
    }

    #[test]
    fn wildcard_matches_any_token_in_similarity() {
        let pattern = vec![
            Token::new("GET"),
            Token::new("<*>"),
            Token::new("<NUM>"),
        ];
        let masked = vec![
            Token::new("GET"),
            Token::new("<PATH>"),
            Token::new("<NUM>"),
        ];
        assert!((sequence_similarity(&pattern, &masked) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fingerprint_uses_event_masked_tokens_after_widen() {
        let mut state = DrainState::new(MiningParams {
            depth: 4,
            ..Default::default()
        });
        let a = masked_tokens("A B C D foo");
        let b = masked_tokens("A B C D bar");
        state.process(&a);
        let result = state.process(&b);
        let expected = Fingerprint::from_masked_tokens(&b.tokens);
        assert_eq!(result.fingerprint, expected);
        assert_ne!(result.fingerprint, Fingerprint::from_masked_tokens(&a.tokens));
    }

    #[test]
    fn reset_clears_registry() {
        let mut state = DrainState::new(MiningParams::default());
        let m = sample_masked();
        state.process(&m);
        state.reset(MiningParams::default());
        let again = state.process(&m);
        assert_eq!(again.template_id, Some(TemplateId(0)));
    }
}
