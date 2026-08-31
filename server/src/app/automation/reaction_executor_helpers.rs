fn event_lines(event: &ExternalEvent) -> Vec<String> {
    [
        ("Repository", attribute(event, "repository")),
        ("Workflow", attribute(event, "workflow")),
        ("Branch", attribute(event, "branch")),
        ("Conclusion", attribute(event, "conclusion")),
        ("Run", attribute(event, "run_url")),
        ("Tag", attribute(event, "tag")),
        ("Commit", attribute(event, "commit_sha")),
        ("Pull request", attribute(event, "pull_request_number")),
        ("Title", attribute(event, "pull_request_title")),
        ("Source branch", attribute(event, "source_branch")),
        ("Actor", attribute(event, "actor")),
        ("Event", attribute(event, "event_url")),
        ("Release", attribute(event, "release_id")),
        ("Release title", attribute(event, "release_title")),
        ("Release state", attribute(event, "release_state")),
        ("Incident", attribute(event, "incident_id")),
        ("Event type", attribute(event, "event_type")),
        ("Source", attribute(event, "source")),
        ("Title", attribute(event, "title")),
        ("Message", attribute(event, "message")),
        ("Severity", attribute(event, "severity")),
        ("External ID", attribute(event, "external_id")),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
    .collect()
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes.saturating_sub('…'.len_utf8());
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value.push('…');
    value
}
