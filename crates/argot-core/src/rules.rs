//! The rule registry — argot's stable, user-facing rule names.
//!
//! Every finding argot can emit belongs to exactly one **rule** (kebab-case,
//! e.g. `foreign-import`), and every rule belongs to one **group** (`voice`,
//! `semantic`, `architecture`, `integrity`). Rules carry a **severity** — `error` (fails
//! `check`), `warn` (reported, does not fail), or `off` (not run) — resolved
//! from, in ascending precedence: registry defaults, `argot.toml [rules]`,
//! `argot.local.toml [rules]`, and CLI `--rule` overrides. Within a layer a
//! rule-specific entry always beats its group entry, whatever the order.
//!
//! The registry is the single source of truth for names, labels, and defaults:
//! config parsing, inline/`[[mute]]` suppressions, the CLI `--rule` flag,
//! `argot rules`, and every output format (human, JSON `rule`, SARIF `ruleId`)
//! resolve through it. Internal scorer *reason codes* (`bpe`, `import`, …) stay
//! private plumbing — hit hashes are computed from them so mutes survive a
//! rename — while everything user-facing speaks rule names.

use std::collections::HashMap;

/// Rule severity — the standard linter triad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    /// Reported and fails `check` (exit 1).
    #[default]
    Error,
    /// Reported, does not fail `check` (unless `--error-on-warnings`).
    Warn,
    /// The rule does not run.
    Off,
}

impl Severity {
    /// Parse a config/CLI severity value. `warning` is accepted as an alias
    /// for `warn` (both vocabularies are common; oxlint accepts both too).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "off" => Some(Self::Off),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Off => "off",
        }
    }
}

/// One registered rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// Stable user-facing name (kebab-case) — config keys, `--rule`,
    /// suppression `rule=`, JSON `rule`, SARIF `ruleId`.
    pub name: &'static str,
    /// Internal scorer reason code this rule maps to (hit hashes use this).
    pub reason: &'static str,
    /// The rule's group (`voice` / `semantic` / `architecture` / `integrity`).
    pub group: &'static str,
    /// Short human label shown next to a finding.
    pub label: &'static str,
    /// One-line description for `argot rules` and docs.
    pub description: &'static str,
    /// Registry default severity — the value `[rules]`/`--rule` layers start
    /// from. Almost everything gates by default; a rule whose accepted-history
    /// false-positive profile is advisory-grade ships as `warn` (decision
    /// recorded in `docs/research/evidence/`).
    pub default_severity: Severity,
}

/// The statistical voice detectors, learned from the repo's git history.
pub const GROUP_VOICE: &str = "voice";
/// The embedding-based detectors (reinvention, placement).
pub const GROUP_SEMANTIC: &str = "semantic";
/// The module-dependency-graph detector.
pub const GROUP_ARCHITECTURE: &str = "architecture";
/// The test-integrity detectors (test gaming: delete / disable / weaken).
pub const GROUP_INTEGRITY: &str = "integrity";

/// Every group name, in display order.
pub const GROUPS: &[&str] = &[
    GROUP_VOICE,
    GROUP_SEMANTIC,
    GROUP_ARCHITECTURE,
    GROUP_INTEGRITY,
];

/// The full registry, in display order. Rules default to `error`
/// ("everything gates by default") except where the accepted-history
/// false-positive profile is advisory-grade (`test-weakened` ships `warn`);
/// users adjust per rule or per group.
pub const RULES: &[Rule] = &[
    Rule {
        name: "foreign-import",
        reason: "import",
        group: GROUP_VOICE,
        label: "foreign import",
        description: "an import of a dependency the repo has never used",
        default_severity: Severity::Error,
    },
    Rule {
        name: "unfamiliar-callee",
        reason: "call_receiver",
        group: GROUP_VOICE,
        label: "unfamiliar callee",
        description: "a call to a receiver or callee the repo's code never calls",
        default_severity: Severity::Error,
    },
    Rule {
        name: "rare-tokens",
        reason: "bpe",
        group: GROUP_VOICE,
        label: "rare token sequence",
        description: "a token sequence statistically foreign to the repo's voice",
        default_severity: Severity::Error,
    },
    Rule {
        name: "convention",
        reason: "convention",
        group: GROUP_VOICE,
        label: "convention",
        description: "a construction that breaks a convention learned from the repo",
        default_severity: Severity::Error,
    },
    Rule {
        name: "redundant",
        reason: "redundant",
        group: GROUP_SEMANTIC,
        label: "already implemented here",
        description: "a new function that duplicates one the repo already has",
        default_severity: Severity::Error,
    },
    Rule {
        name: "misplaced",
        reason: "misplaced",
        group: GROUP_SEMANTIC,
        label: "unusual location",
        description: "a function that looks like it belongs in another module area",
        default_severity: Severity::Error,
    },
    Rule {
        name: "layering",
        reason: "layering",
        group: GROUP_ARCHITECTURE,
        label: "crosses a module boundary",
        description: "an internal import that reverses the repo's layer direction",
        default_severity: Severity::Error,
    },
    Rule {
        name: "test-deleted",
        reason: "test_deleted",
        group: GROUP_INTEGRITY,
        label: "test deleted alongside code change",
        description: "a test removed while the production code it exercised still exists",
        default_severity: Severity::Error,
    },
    Rule {
        name: "test-disabled",
        reason: "test_disabled",
        group: GROUP_INTEGRITY,
        label: "test disabled alongside code change",
        description: "a skip/ignore marker added or a test gutted while production code changes",
        default_severity: Severity::Error,
    },
    Rule {
        name: "test-weakened",
        reason: "test_weakened",
        group: GROUP_INTEGRITY,
        label: "test weakened alongside code change",
        description: "assertions removed, tautologized, or loosened while production code changes",
        default_severity: Severity::Warn,
    },
];

/// The rule a scorer reason code belongs to (`None` for internal, non-flagging
/// reasons like `none` / `auto_generated`).
pub fn rule_for_reason(reason: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.reason == reason)
}

/// Look a rule up by its user-facing name.
pub fn rule_named(name: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.name == name)
}

/// Is `name` a group name?
pub fn is_group(name: &str) -> bool {
    GROUPS.contains(&name)
}

/// The user-facing rule code for a reason: the rule name when the reason is
/// registered, the raw reason otherwise (internal codes like `none` surface
/// as-is under `--threshold` overrides).
pub fn code_for_reason(reason: &str) -> &str {
    rule_for_reason(reason).map(|r| r.name).unwrap_or(reason)
}

/// The human label for a reason (falls back to the raw reason).
pub fn label_for_reason(reason: &str) -> &str {
    rule_for_reason(reason).map(|r| r.label).unwrap_or(reason)
}

/// Does `selector` (a rule name or group name) cover the rule behind `reason`?
/// Used by the suppression surfaces (`rule=` inline scopes, `[[mute]].rule`).
pub fn selector_matches_reason(selector: &str, reason: &str) -> bool {
    rule_for_reason(reason).is_some_and(|r| r.name == selector || r.group == selector)
}

/// A valid `rule=` / `[[mute]].rule` / `--rule` / `[rules]` key?
pub fn known_selector(name: &str) -> bool {
    is_group(name) || rule_named(name).is_some()
}

/// Every selector a user can write, for error messages.
pub fn selector_names() -> Vec<&'static str> {
    GROUPS
        .iter()
        .copied()
        .chain(RULES.iter().map(|r| r.name))
        .collect()
}

/// One configuration layer: validated `(selector, severity)` entries from a
/// single source (`argot.toml`, `argot.local.toml`, or the CLI).
pub type RulesLayer = Vec<(String, Severity)>;

/// The resolved per-rule severities.
#[derive(Debug, Clone, Default)]
pub struct RuleSettings {
    by_reason: HashMap<&'static str, Severity>,
}

impl RuleSettings {
    /// Resolve severities from layers in ascending precedence (defaults, then
    /// each layer in order — config base, config local, CLI). Within a layer a
    /// rule-specific entry beats a group entry regardless of order.
    pub fn resolve(layers: &[RulesLayer]) -> Self {
        let mut by_reason = HashMap::new();
        for rule in RULES {
            let mut sev = rule.default_severity;
            for layer in layers {
                let group = layer
                    .iter()
                    .rev()
                    .find(|(k, _)| k == rule.group)
                    .map(|(_, s)| *s);
                let specific = layer
                    .iter()
                    .rev()
                    .find(|(k, _)| k == rule.name)
                    .map(|(_, s)| *s);
                if let Some(s) = specific.or(group) {
                    sev = s;
                }
            }
            by_reason.insert(rule.reason, sev);
        }
        RuleSettings { by_reason }
    }

    /// The severity of the rule behind a reason code. Unregistered reasons
    /// (internal codes, future rules) resolve to `error` — a finding must
    /// never silently lose its gate because the registry lags.
    pub fn severity_of_reason(&self, reason: &str) -> Severity {
        self.by_reason
            .get(reason)
            .copied()
            .unwrap_or(Severity::Error)
    }

    /// The severity of a rule by name.
    pub fn severity_of_rule(&self, rule: &Rule) -> Severity {
        self.severity_of_reason(rule.reason)
    }

    /// True when at least one rule in `group` runs (severity != off) — the
    /// gate for skipping a whole detector pass (and its costs: index load,
    /// model download).
    pub fn group_enabled(&self, group: &str) -> bool {
        RULES
            .iter()
            .filter(|r| r.group == group)
            .any(|r| self.severity_of_reason(r.reason) != Severity::Off)
    }
}

/// Validate raw `(key, value)` entries into a [`RulesLayer`]. Unknown keys and
/// invalid severities produce a warning (`origin` names the source file) and
/// are skipped — config degrades, it never fails the run.
pub fn validate_layer(
    raw: &[(String, String)],
    origin: &str,
    warnings: &mut Vec<String>,
) -> RulesLayer {
    let mut layer = RulesLayer::new();
    for (key, value) in raw {
        if !known_selector(key) {
            warnings.push(format!(
                "{origin}: [rules] unknown rule '{key}' — ignored (known: {})",
                selector_names().join(", ")
            ));
            continue;
        }
        match Severity::parse(value) {
            Some(sev) => layer.push((key.clone(), sev)),
            None => warnings.push(format!(
                "{origin}: [rules] invalid severity '{value}' for '{key}' — \
                 expected error, warn, or off; ignored"
            )),
        }
    }
    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_are_unique_and_kebab_case() {
        let mut seen = std::collections::HashSet::new();
        for r in RULES {
            assert!(seen.insert(r.name), "duplicate rule name {}", r.name);
            assert!(
                r.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()),
                "{} is not kebab-case",
                r.name
            );
            assert!(GROUPS.contains(&r.group), "{} has unknown group", r.name);
        }
        // A rule name must never shadow a group name.
        for r in RULES {
            assert!(!is_group(r.name), "{} shadows a group name", r.name);
        }
    }

    #[test]
    fn defaults_follow_the_registry() {
        let s = RuleSettings::resolve(&[]);
        for r in RULES {
            assert_eq!(s.severity_of_reason(r.reason), r.default_severity);
        }
        // Everything gates by default except the advisory-profiled rule.
        assert_eq!(s.severity_of_reason("test_weakened"), Severity::Warn);
        assert_eq!(s.severity_of_reason("test_deleted"), Severity::Error);
        assert_eq!(s.severity_of_reason("import"), Severity::Error);
        assert!(s.group_enabled(GROUP_VOICE));
        assert!(s.group_enabled(GROUP_SEMANTIC));
        assert!(s.group_enabled(GROUP_ARCHITECTURE));
        assert!(s.group_enabled(GROUP_INTEGRITY));
    }

    #[test]
    fn rule_specific_beats_group_within_a_layer() {
        // Whatever the order, `redundant = warn` beats `semantic = off`.
        for layer in [
            vec![
                ("semantic".to_string(), Severity::Off),
                ("redundant".to_string(), Severity::Warn),
            ],
            vec![
                ("redundant".to_string(), Severity::Warn),
                ("semantic".to_string(), Severity::Off),
            ],
        ] {
            let s = RuleSettings::resolve(&[layer]);
            assert_eq!(s.severity_of_reason("redundant"), Severity::Warn);
            assert_eq!(s.severity_of_reason("misplaced"), Severity::Off);
            assert!(s.group_enabled(GROUP_SEMANTIC), "redundant still runs");
        }
    }

    #[test]
    fn later_layers_override_earlier_ones() {
        let base = vec![("semantic".to_string(), Severity::Off)];
        let cli = vec![("redundant".to_string(), Severity::Error)];
        let s = RuleSettings::resolve(&[base, cli]);
        assert_eq!(s.severity_of_reason("redundant"), Severity::Error);
        assert_eq!(s.severity_of_reason("misplaced"), Severity::Off);
    }

    #[test]
    fn group_disabled_only_when_every_rule_is_off() {
        let s = RuleSettings::resolve(&[vec![
            ("redundant".to_string(), Severity::Off),
            ("misplaced".to_string(), Severity::Off),
        ]]);
        assert!(!s.group_enabled(GROUP_SEMANTIC));
        assert!(s.group_enabled(GROUP_VOICE));
    }

    #[test]
    fn unknown_reason_is_error() {
        let s = RuleSettings::resolve(&[vec![("voice".to_string(), Severity::Off)]]);
        assert_eq!(s.severity_of_reason("none"), Severity::Error);
        assert_eq!(s.severity_of_reason("structural"), Severity::Error);
    }

    #[test]
    fn selector_matching_covers_names_and_groups() {
        assert!(selector_matches_reason("foreign-import", "import"));
        assert!(selector_matches_reason("voice", "import"));
        assert!(selector_matches_reason("semantic", "redundant"));
        assert!(!selector_matches_reason("semantic", "import"));
        assert!(!selector_matches_reason("foreign-import", "bpe"));
        // Internal reasons match no selector.
        assert!(!selector_matches_reason("voice", "none"));
    }

    #[test]
    fn severity_parse_accepts_warning_alias() {
        assert_eq!(Severity::parse("warn"), Some(Severity::Warn));
        assert_eq!(Severity::parse("warning"), Some(Severity::Warn));
        assert_eq!(Severity::parse("error"), Some(Severity::Error));
        assert_eq!(Severity::parse("off"), Some(Severity::Off));
        assert_eq!(Severity::parse("deny"), None);
    }

    #[test]
    fn validate_layer_warns_on_unknown_and_invalid() {
        let mut warnings = Vec::new();
        let layer = validate_layer(
            &[
                ("redundant".to_string(), "warn".to_string()),
                ("quantum".to_string(), "off".to_string()),
                ("misplaced".to_string(), "loud".to_string()),
            ],
            "argot.toml",
            &mut warnings,
        );
        assert_eq!(layer, vec![("redundant".to_string(), Severity::Warn)]);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("unknown rule 'quantum'"));
        assert!(warnings[1].contains("invalid severity 'loud'"));
    }

    #[test]
    fn code_and_label_fall_back_to_raw_reason() {
        assert_eq!(code_for_reason("import"), "foreign-import");
        assert_eq!(label_for_reason("bpe"), "rare token sequence");
        assert_eq!(code_for_reason("none"), "none");
        assert_eq!(label_for_reason("none"), "none");
    }
}
