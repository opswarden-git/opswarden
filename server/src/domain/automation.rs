// --- server/src/domain/automation.rs ---
//
// The Action -> REAction core, kept pure. The domain knows what an external
// signal is and which rules fire for it; it knows nothing about HTTP, HMAC, JSON
// or which provider sent the webhook. Adapters decode a provider payload into an
// `ExternalEvent`; this module decides what should happen.

use uuid::Uuid;

use super::incident::Severity;

/// A normalized external signal, decoded from a provider webhook. Pure value: the
/// receiving adapter maps the raw provider payload (GitHub `workflow_run`, …) onto
/// this `service` + `kind` pair so the rule engine stays provider-agnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEvent {
    /// The source service, e.g. `"github"`.
    pub service: String,
    /// The normalized event kind, e.g. `"ci_failed"`.
    pub kind: String,
}

impl ExternalEvent {
    pub fn new(service: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            kind: kind.into(),
        }
    }
}

/// What a rule does when its trigger fires (the "REAction").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction {
    /// Open an incident in `team_id` with the given severity and title.
    CreateIncident {
        team_id: Uuid,
        severity: Severity,
        title: String,
    },
    /// POST a notification to an outbound URL. Generic on purpose: a Slack
    /// incoming-webhook URL (the body is `{"text": ...}`), Discord, Teams or any
    /// HTTP endpoint are all just a `url` here — one connector, many targets.
    Notify {
        team_id: Uuid,
        url: String,
        message: String,
    },
}

impl Reaction {
    /// The team this reaction acts within — used to scope the broadcast of the
    /// `rule_triggered` / `rule_failed` event to that team's members.
    pub fn team_id(&self) -> Uuid {
        match self {
            Reaction::CreateIncident { team_id, .. } | Reaction::Notify { team_id, .. } => *team_id,
        }
    }
}

/// An automation rule: when an event matching the trigger arrives, run `reaction`.
/// Filters are intentionally minimal for now (service + kind exact match); richer
/// filtering plugs in here without touching the engine or the adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Stable identifier surfaced on the `rule_triggered` / `rule_failed` event.
    pub name: String,
    pub on_service: String,
    pub on_kind: String,
    pub reaction: Reaction,
}

impl Rule {
    pub fn matches(&self, event: &ExternalEvent) -> bool {
        self.on_service == event.service && self.on_kind == event.kind
    }
}

/// Pure rule evaluation: the rules whose trigger matches `event`, in declared
/// order. No I/O, fully unit-testable.
pub fn evaluate<'a>(rules: &'a [Rule], event: &ExternalEvent) -> Vec<&'a Rule> {
    rules.iter().filter(|rule| rule.matches(event)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ci_failed_rule(team_id: Uuid) -> Rule {
        Rule {
            name: "github-ci-failed-to-incident".to_string(),
            on_service: "github".to_string(),
            on_kind: "ci_failed".to_string(),
            reaction: Reaction::CreateIncident {
                team_id,
                severity: Severity::High,
                title: "CI failed on GitHub".to_string(),
            },
        }
    }

    #[test]
    fn rule_matches_only_its_service_and_kind() {
        let rule = ci_failed_rule(Uuid::new_v4());

        assert!(rule.matches(&ExternalEvent::new("github", "ci_failed")));
        assert!(!rule.matches(&ExternalEvent::new("github", "ci_passed")));
        assert!(!rule.matches(&ExternalEvent::new("gitlab", "ci_failed")));
    }

    #[test]
    fn evaluate_returns_matching_rules_in_order() {
        let team = Uuid::new_v4();
        let rules = vec![ci_failed_rule(team)];

        let matched = evaluate(&rules, &ExternalEvent::new("github", "ci_failed"));
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "github-ci-failed-to-incident");
    }

    #[test]
    fn evaluate_returns_nothing_when_no_rule_matches() {
        let rules = vec![ci_failed_rule(Uuid::new_v4())];

        let matched = evaluate(&rules, &ExternalEvent::new("github", "push"));
        assert!(matched.is_empty());
    }

    #[test]
    fn reaction_exposes_its_team_for_event_scoping() {
        let team = Uuid::new_v4();
        let rule = ci_failed_rule(team);
        assert_eq!(rule.reaction.team_id(), team);

        let notify = Reaction::Notify {
            team_id: team,
            url: "https://hooks.slack.com/services/xxx".to_string(),
            message: "CI failed".to_string(),
        };
        assert_eq!(notify.team_id(), team);
    }
}
