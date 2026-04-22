use std::{path::Path, process::Command};

use serde::Deserialize;
use serde_json::Value;

use crate::model::{Task, TaskStatus, TrailEvent, TrailEventKind};

use super::SourceLoadResult;

pub fn detect(root: &Path) -> bool {
    root.join(".beads").exists()
}

pub fn load(root: &Path) -> Result<SourceLoadResult, String> {
    let output = Command::new("bd")
        .args(["list", "--json", "--sandbox", "--no-daemon"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to execute `bd list --json`: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_owned());
    }

    let issues: Vec<BeadIssue> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("failed to parse `bd` JSON: {error}"))?;
    let warnings = parse_warnings(&output.stderr);
    let trail_events = load_activity(root).unwrap_or_default();

    let tasks = issues
        .into_iter()
        .map(|issue| Task {
            id: issue.id,
            title: issue.title,
            status: TaskStatus::from_raw(&issue.status),
            source: "beads".to_owned(),
            source_path: Some(root.join(".beads").display().to_string()),
            assignee: issue.owner,
            claimed_at: if issue.status.eq_ignore_ascii_case("in_progress") {
                issue.updated_at.clone()
            } else {
                None
            },
            completed_at: issue.closed_at.clone(),
            updated_at: issue.updated_at,
            dependency_ids: issue
                .dependencies
                .unwrap_or_default()
                .into_iter()
                .map(|dependency| dependency.depends_on_id)
                .collect(),
            dependent_ids: Vec::new(),
            url: None,
        })
        .collect();

    Ok(SourceLoadResult {
        tasks,
        trail_events,
        warnings,
    })
}

fn load_activity(root: &Path) -> Result<Vec<TrailEvent>, String> {
    let output = Command::new("bd")
        .args(["activity", "--details", "--json", "--limit", "400", "--since", "720h"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to execute `bd activity --json`: {error}"))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    parse_activity_events(&output.stdout)
}

fn parse_activity_events(stdout: &[u8]) -> Result<Vec<TrailEvent>, String> {
    let values: Vec<Value> = serde_json::from_slice(stdout)
        .map_err(|error| format!("failed to parse `bd activity` JSON: {error}"))?;
    let mut events = Vec::new();

    for value in values {
        let Some(kind) = classify_activity_kind(&value) else {
            continue;
        };
        let Some(task_id) = extract_task_id(&value) else {
            continue;
        };
        let Some(timestamp) = find_first_string(
            &value,
            &["timestamp", "occurred_at", "created_at", "updated_at", "closed_at", "at", "time"],
        ) else {
            continue;
        };
        let actor = find_first_string(&value, &["actor", "owner", "created_by", "updated_by"])
            .unwrap_or("shared")
            .to_owned();

        events.push(TrailEvent {
            task_id: task_id.to_owned(),
            actor,
            timestamp: timestamp.to_owned(),
            kind,
        });
    }

    events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    Ok(events)
}

fn classify_activity_kind(value: &Value) -> Option<TrailEventKind> {
    let strings = collect_strings(value);

    if strings
        .iter()
        .any(|value| value == "→" || value.contains("in_progress") || value.contains("started") || value.contains("working"))
    {
        return Some(TrailEventKind::Claimed);
    }

    if strings
        .iter()
        .any(|value| value == "✓" || value.contains("complete") || value.contains("closed") || value.contains("done"))
    {
        return Some(TrailEventKind::Completed);
    }

    None
}

fn extract_task_id(value: &Value) -> Option<&str> {
    value.get("issue")
        .and_then(|issue| issue.get("id"))
        .and_then(Value::as_str)
        .or_else(|| find_first_string(value, &["issue_id", "id"]))
}

fn find_first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_str) {
                    return Some(found);
                }
            }
            for nested in map.values() {
                if let Some(found) = find_first_string(nested, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(values) => values
            .iter()
            .find_map(|nested| find_first_string(nested, keys)),
        _ => None,
    }
}

fn collect_strings(value: &Value) -> Vec<String> {
    let mut strings = Vec::new();
    collect_strings_into(value, &mut strings);
    strings
}

fn collect_strings_into(value: &Value, strings: &mut Vec<String>) {
    match value {
        Value::String(text) => strings.push(text.to_ascii_lowercase()),
        Value::Array(values) => {
            for nested in values {
                collect_strings_into(nested, strings);
            }
        }
        Value::Object(map) => {
            for nested in map.values() {
                collect_strings_into(nested, strings);
            }
        }
        _ => {}
    }
}

fn parse_warnings(stderr: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("Warning: Daemon took too long")
                && !trimmed.starts_with("Hint: Run 'bd doctor'")
        })
        .map(|line| line.trim().to_owned())
        .collect()
}

#[derive(Debug, Deserialize)]
struct BeadIssue {
    id: String,
    title: String,
    status: String,
    owner: Option<String>,
    closed_at: Option<String>,
    updated_at: Option<String>,
    dependencies: Option<Vec<BeadDependency>>,
}

#[derive(Debug, Deserialize)]
struct BeadDependency {
    depends_on_id: String,
}
