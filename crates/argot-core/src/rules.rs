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

/// The reserved group name for runtime-loaded (community) rules, and the
/// namespace prefix of their reason codes: a custom rule `<name>` emits
/// findings with reason `custom:<name>`. The prefix keeps custom reasons
/// collision-proof against internal codes and makes the reason ↔ rule-name
/// mapping syntactic — the suppression hot paths never need a registry.
pub const GROUP_CUSTOM: &str = "custom";
/// The reason-code prefix for custom rules (see [`GROUP_CUSTOM`]).
pub const CUSTOM_REASON_PREFIX: &str = "custom:";

/// The custom rule name behind a reason code, when the reason is in the
/// custom namespace.
fn custom_name_of_reason(reason: &str) -> Option<&str> {
    reason.strip_prefix(CUSTOM_REASON_PREFIX)
}

/// One runtime-registered rule (loaded from `.argot/rules/<name>/`), group
/// [`GROUP_CUSTOM`]. The built-in vocabulary stays the static [`RULES`]
/// table — always known, feature-independent — while custom rules overlay it
/// per run through [`Registry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomRule {
    /// Stable user-facing name (kebab-case) — same surfaces as a built-in
    /// rule name. Reason code is derived: `custom:<name>`.
    pub name: String,
    /// Short human label shown next to a finding.
    pub label: String,
    /// One-line description for `argot rules`.
    pub description: String,
    /// The manifest's default severity (overridable via `[rules]`/`--rule`).
    pub default_severity: Severity,
}

/// The run's rule vocabulary: the static built-in table plus any custom rules
/// discovered at startup. Engine paths that must know every rule (config
/// validation, severity resolution, output labels) resolve through this;
/// paths where the mapping is syntactic (suppression matching) keep using the
/// free functions.
#[derive(Debug, Default)]
pub struct Registry {
    custom: Vec<CustomRule>,
}

impl Registry {
    /// The built-ins-only registry (no custom rules).
    pub fn builtin() -> &'static Registry {
        static BUILTIN: Registry = Registry { custom: Vec::new() };
        &BUILTIN
    }

    /// Overlay validated custom rules onto the built-in vocabulary. Rejects
    /// (per rule, with a message) non-kebab-case names and collisions with
    /// built-in rule names, group names, or an earlier custom rule; rejected
    /// rules are skipped so one bad manifest never takes down the run.
    pub fn with_custom(rules: Vec<CustomRule>, warnings: &mut Vec<String>) -> Registry {
        let mut custom: Vec<CustomRule> = Vec::new();
        for rule in rules {
            let name = rule.name.as_str();
            let kebab = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit());
            if !kebab {
                warnings.push(format!(
                    "custom rule '{name}': not kebab-case — skipped (lowercase, digits, '-')"
                ));
                continue;
            }
            if rule_named(name).is_some() || is_group(name) || name == GROUP_CUSTOM {
                warnings.push(format!(
                    "custom rule '{name}': collides with a built-in rule or group name — skipped"
                ));
                continue;
            }
            if custom.iter().any(|c| c.name == name) {
                warnings.push(format!(
                    "custom rule '{name}': duplicate of an earlier custom rule — skipped"
                ));
                continue;
            }
            custom.push(rule);
        }
        Registry { custom }
    }

    /// The registered custom rules, in registration order.
    pub fn custom_rules(&self) -> &[CustomRule] {
        &self.custom
    }

    /// Custom-rule lookup by user-facing name.
    pub fn custom_named(&self, name: &str) -> Option<&CustomRule> {
        self.custom.iter().find(|c| c.name == name)
    }

    /// A valid `[rules]` / `rule=` / `--rule` key against this vocabulary?
    pub fn known_selector(&self, name: &str) -> bool {
        known_selector(name)
            || name == GROUP_CUSTOM
            || self.custom_named(name).is_some()
    }

    /// Every selector a user can write, for error messages. Groups first
    /// (custom last), then rules in display order.
    pub fn selector_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = GROUPS.to_vec();
        names.push(GROUP_CUSTOM);
        names.extend(RULES.iter().map(|r| r.name));
        names.extend(self.custom.iter().map(|c| c.name.as_str()));
        names
    }

    /// The human label for a reason — built-in table first, then the custom
    /// namespace, then the raw reason.
    pub fn label_for_reason<'a>(&'a self, reason: &'a str) -> &'a str {
        if let Some(rule) = rule_for_reason(reason) {
            return rule.label;
        }
        custom_name_of_reason(reason)
            .and_then(|n| self.custom_named(n))
            .map(|c| c.label.as_str())
            .unwrap_or(reason)
    }
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
/// registered, the custom rule name for `custom:` reasons (syntactic — the
/// namespace prefix IS the mapping), the raw reason otherwise (internal codes
/// like `none` surface as-is under `--threshold` overrides).
pub fn code_for_reason(reason: &str) -> &str {
    if let Some(rule) = rule_for_reason(reason) {
        return rule.name;
    }
    custom_name_of_reason(reason).unwrap_or(reason)
}

/// The human label for a reason (falls back to the raw reason).
pub fn label_for_reason(reason: &str) -> &str {
    rule_for_reason(reason).map(|r| r.label).unwrap_or(reason)
}

/// Does `selector` (a rule name or group name) cover the rule behind `reason`?
/// Used by the suppression surfaces (`rule=` inline scopes, `[[mute]].rule`).
/// Custom reasons match syntactically: their rule name is the reason minus
/// the `custom:` prefix, their group is always `custom`.
pub fn selector_matches_reason(selector: &str, reason: &str) -> bool {
    if let Some(name) = custom_name_of_reason(reason) {
        return selector == name || selector == GROUP_CUSTOM;
    }
    rule_for_reason(reason).is_some_and(|r| r.name == selector || r.group == selector)
}

/// A valid `rule=` / `[[mute]].rule` / `--rule` / `[rules]` key? The `custom`
/// group is always valid vocabulary — it scopes whatever custom rules a repo
/// carries, and a selector's validity must not depend on which repo the
/// command runs in.
pub fn known_selector(name: &str) -> bool {
    is_group(name) || name == GROUP_CUSTOM || rule_named(name).is_some()
}

/// Every selector a user can write, for error messages.
pub fn selector_names() -> Vec<&'static str> {
    GROUPS
        .iter()
        .copied()
        .chain([GROUP_CUSTOM])
        .chain(RULES.iter().map(|r| r.name))
        .collect()
}

/// One configuration layer: validated `(selector, severity)` entries from a
/// single source (`argot.toml`, `argot.local.toml`, or the CLI).
pub type RulesLayer = Vec<(String, Severity)>;

/// The resolved per-rule severities.
#[derive(Debug, Clone, Default)]
pub struct RuleSettings {
    by_reason: HashMap<String, Severity>,
    /// Reason codes of the run's custom rules (for `group_enabled(custom)`).
    custom_reasons: Vec<String>,
}

impl RuleSettings {
    /// Resolve severities from layers in ascending precedence (defaults, then
    /// each layer in order — config base, config local, CLI). Within a layer a
    /// rule-specific entry beats a group entry regardless of order.
    pub fn resolve(layers: &[RulesLayer]) -> Self {
        Self::resolve_with(Registry::builtin(), layers)
    }

    /// [`RuleSettings::resolve`] against a run vocabulary: built-in rules plus
    /// the registry's custom rules (group `custom`, defaults from their
    /// manifests).
    pub fn resolve_with(registry: &Registry, layers: &[RulesLayer]) -> Self {
        let resolve_one = |name: &str, group: &str, default: Severity| {
            let mut sev = default;
            for layer in layers {
                let group_entry = layer
                    .iter()
                    .rev()
                    .find(|(k, _)| k == group)
                    .map(|(_, s)| *s);
                let specific = layer.iter().rev().find(|(k, _)| k == name).map(|(_, s)| *s);
                if let Some(s) = specific.or(group_entry) {
                    sev = s;
                }
            }
            sev
        };
        let mut by_reason = HashMap::new();
        for rule in RULES {
            by_reason.insert(
                rule.reason.to_string(),
                resolve_one(rule.name, rule.group, rule.default_severity),
            );
        }
        let mut custom_reasons = Vec::new();
        for rule in registry.custom_rules() {
            let reason = format!("{CUSTOM_REASON_PREFIX}{}", rule.name);
            by_reason.insert(
                reason.clone(),
                resolve_one(&rule.name, GROUP_CUSTOM, rule.default_severity),
            );
            custom_reasons.push(reason);
        }
        RuleSettings {
            by_reason,
            custom_reasons,
        }
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
        if group == GROUP_CUSTOM {
            return self
                .custom_reasons
                .iter()
                .any(|r| self.severity_of_reason(r) != Severity::Off);
        }
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
    validate_layer_with(Registry::builtin(), raw, origin, warnings)
}

/// [`validate_layer`] against a run vocabulary (built-ins + custom rules).
pub fn validate_layer_with(
    registry: &Registry,
    raw: &[(String, String)],
    origin: &str,
    warnings: &mut Vec<String>,
) -> RulesLayer {
    let mut layer = RulesLayer::new();
    for (key, value) in raw {
        if !registry.known_selector(key) {
            warnings.push(format!(
                "{origin}: [rules] unknown rule '{key}' — ignored (known: {})",
                registry.selector_names().join(", ")
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

    fn custom(name: &str) -> CustomRule {
        CustomRule {
            name: name.to_string(),
            label: format!("{name} label"),
            description: format!("{name} description"),
            default_severity: Severity::Warn,
        }
    }

    #[test]
    fn custom_reasons_map_syntactically() {
        assert_eq!(code_for_reason("custom:raw-sql"), "raw-sql");
        assert!(selector_matches_reason("raw-sql", "custom:raw-sql"));
        assert!(selector_matches_reason("custom", "custom:raw-sql"));
        assert!(!selector_matches_reason("voice", "custom:raw-sql"));
        assert!(!selector_matches_reason("other-rule", "custom:raw-sql"));
    }

    #[test]
    fn custom_group_is_always_valid_vocabulary() {
        assert!(known_selector("custom"));
        assert!(selector_names().contains(&"custom"));
        // But a rule name must still not shadow it (with_custom rejects).
        let mut warnings = Vec::new();
        let reg = Registry::with_custom(vec![custom("custom")], &mut warnings);
        assert!(reg.custom_rules().is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn with_custom_rejects_collisions_and_bad_names() {
        let mut warnings = Vec::new();
        let reg = Registry::with_custom(
            vec![
                custom("raw-sql"),
                custom("raw-sql"),        // duplicate
                custom("foreign-import"), // shadows a built-in rule
                custom("voice"),          // shadows a group
                custom("Bad_Name"),       // not kebab-case
            ],
            &mut warnings,
        );
        assert_eq!(
            reg.custom_rules().iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["raw-sql"]
        );
        assert_eq!(warnings.len(), 4);
        assert!(reg.known_selector("raw-sql"));
        assert!(!reg.known_selector("unregistered-rule"));
        assert_eq!(reg.label_for_reason("custom:raw-sql"), "raw-sql label");
        assert_eq!(reg.label_for_reason("custom:unknown"), "custom:unknown");
        assert_eq!(reg.label_for_reason("bpe"), "rare token sequence");
    }

    #[test]
    fn custom_severities_resolve_through_the_same_layers() {
        let mut warnings = Vec::new();
        let reg = Registry::with_custom(vec![custom("raw-sql")], &mut warnings);
        assert!(warnings.is_empty());

        // Manifest default applies with no layers.
        let s = RuleSettings::resolve_with(&reg, &[]);
        assert_eq!(s.severity_of_reason("custom:raw-sql"), Severity::Warn);
        assert!(s.group_enabled(GROUP_CUSTOM));

        // Rule-specific beats the `custom` group entry within a layer.
        let layer = vec![
            ("custom".to_string(), Severity::Off),
            ("raw-sql".to_string(), Severity::Error),
        ];
        let s = RuleSettings::resolve_with(&reg, &[layer]);
        assert_eq!(s.severity_of_reason("custom:raw-sql"), Severity::Error);

        // Group off with no specific → the whole custom pass can be skipped.
        let s = RuleSettings::resolve_with(&reg, &[vec![("custom".to_string(), Severity::Off)]]);
        assert_eq!(s.severity_of_reason("custom:raw-sql"), Severity::Off);
        assert!(!s.group_enabled(GROUP_CUSTOM));
        // Built-in groups are untouched by the custom layer.
        assert!(s.group_enabled(GROUP_VOICE));
    }

    #[test]
    fn validate_layer_with_accepts_custom_keys() {
        let mut warnings = Vec::new();
        let reg = Registry::with_custom(vec![custom("raw-sql")], &mut warnings);
        let layer = validate_layer_with(
            &reg,
            &[
                ("raw-sql".to_string(), "error".to_string()),
                ("unknown-thing".to_string(), "off".to_string()),
            ],
            "argot.toml",
            &mut warnings,
        );
        assert_eq!(layer, vec![("raw-sql".to_string(), Severity::Error)]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown rule 'unknown-thing'"));
        assert!(warnings[0].contains("raw-sql"), "custom rule listed as known");
    }

    #[test]
    fn builtin_registry_matches_free_functions() {
        let reg = Registry::builtin();
        assert!(reg.custom_rules().is_empty());
        for r in RULES {
            assert!(reg.known_selector(r.name));
            assert_eq!(reg.label_for_reason(r.reason), r.label);
        }
        assert_eq!(reg.selector_names(), selector_names());
    }
}
