use super::*;
use std::path::PathBuf;

fn camel_corpus() -> Vec<(PathBuf, String)> {
    // Two templates × 10 files: arrow/const/typed-object style, no `var`,
    // no classes — a plausible modern-TS voice in miniature.
    (0..20)
        .map(|i| {
            let src = if i % 2 == 0 {
                "export function computeTotal(itemValues: number[]): number {\n  \
                 const runningTotal = itemValues.reduce((acc, item) => acc + item, 0)\n  \
                 return runningTotal\n}\n"
            } else {
                "export const describeBox = (box: { value: number }): string => {\n  \
                 const label = `box-${box.value}`\n  if (box.value > 0) {\n    \
                 return label\n  }\n  return label.trim()\n}\n"
            };
            (PathBuf::from(format!("m{i}.ts")), src.to_string())
        })
        .collect()
}

#[test]
fn shape_classification() {
    assert_eq!(ident_shape("computeTotal"), "camel");
    assert_eq!(ident_shape("ComputeTotal"), "pascal");
    assert_eq!(ident_shape("compute_total"), "snake");
    assert_eq!(ident_shape("COMPUTE_TOTAL"), "scream");
    assert_eq!(ident_shape("compute"), "flat");
}

#[test]
fn snake_case_in_camel_corpus_scores_high_ident_surprisal() {
    let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
    let scorer = ConventionScorer::new(model, Language::Typescript);
    let alien = "function compute_weighted_sum(input_values: number[]) {\n  \
                 let total_sum = 0\n  let weight_sum = 0\n  return total_sum + weight_sum\n}";
    let native = "function computeWeightedSum(inputValues: number[]) {\n  \
                  let totalSum = 0\n  let weightSum = 0\n  return totalSum + weightSum\n}";
    let a = scorer.ident_surprisal(alien);
    let n = scorer.ident_surprisal(native);
    assert!(a > n + 1.0, "snake surprisal {a} vs camel {n}");
}

#[test]
fn unused_construct_scores_high_syntax_surprisal() {
    let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
    let scorer = ConventionScorer::new(model, Language::Typescript);
    // The corpus has no `var` declarations and no classes.
    let alien = "export class LegacyBox {\n  private value = 0\n  \
                 public getValue(): number {\n    var copy = this.value\n    \
                 var doubled = copy * 2\n    var label = String(doubled)\n    \
                 return label.length\n  }\n}";
    let native = "export function getValue(box: { value: number }): number {\n  \
                  const copy = box.value\n  return copy\n}";
    let a = scorer.scores(alien, None);
    let n = scorer.scores(native, None);
    assert!(
        a.syntax_surprisal > n.syntax_surprisal + 2.0,
        "class/var {a:?} vs function/const {n:?}"
    );
}

#[test]
fn parse_error_fragment_uses_host_region() {
    let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
    let scorer = ConventionScorer::new(model, Language::Typescript);
    // A fragment starting with a stray `}` has root errors; its kinds must
    // come from the host region, not the broken bare parse.
    let fragment = "}\n\nexport class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}";
    let host = "export function ok() {\n  return 1\n}\n\nexport class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}\n";
    let with_host = scorer.scores(fragment, Some((host, 3, 10)));
    let bare_ok = scorer.scores(
        "export class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}",
        None,
    );
    assert!(
        (with_host.syntax_surprisal - bare_ok.syntax_surprisal).abs() < 1.0,
        "host-region kinds ≈ clean-fragment kinds: {with_host:?} vs {bare_ok:?}"
    );
}

#[test]
fn bars_gate_firing() {
    let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
    let mut scorer = ConventionScorer::new(model, Language::Typescript);
    let alien = "function compute_weighted_sum(input_values: number[]) {\n  \
                 let total_sum = 0\n  let weight_sum = 0\n  return total_sum + weight_sum\n}";
    let s = scorer.scores(alien, None);
    assert!(!scorer.fires(&s), "uncalibrated bars never fire");
    // Set every present shape's bar just below its surprisal so it fires.
    let bars = s
        .ident_surprisals
        .iter()
        .map(|(shape, &v)| (shape.clone(), v - 0.1))
        .collect();
    scorer.set_bars(1000.0, bars);
    assert!(scorer.fires(&s), "bar below the score fires");
}
