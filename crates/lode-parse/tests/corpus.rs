//! Golden corpus integration tests — fixture loading via serde/toml (T0.3).

mod corpus_loader;

use corpus_loader::{corpus_root, load_corpus};

#[test]
fn corpus_fixtures_load() {
    let input = load_corpus(corpus_root());
    assert_eq!(input.formats.len(), 3);
    let total_lines: usize = input.formats.iter().map(|f| f.lines.len()).sum();
    assert_eq!(total_lines, 165);
    for format in &input.formats {
        assert_eq!(format.lines.len(), format.labels.len());
        assert!((format.spec.pa_floor - 0.90).abs() < f64::EPSILON);
    }
}

#[test]
#[ignore = "gate T1.4: requires DrainMiner — only real mining may assert PA floor"]
fn corpus_pa_meets_floor() {
    let input = load_corpus(corpus_root());
    // TODO(T1.4): let mut miner = DrainMiner::default();
    // let result = run_corpus(&input, &mut miner).expect("evaluate");
    // for format in &input.formats {
    //     let pa = result.per_format_pa[&format.spec.id];
    //     assert!(pa >= format.spec.pa_floor, "{}", format.spec.id);
    // }
    let _ = input;
}
