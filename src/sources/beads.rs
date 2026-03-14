use std::{path::Path, process::Command};

use serde::Deserialize;

use crate::model::{Task, TaskStatus};

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

    let tasks = issues
        .into_iter()
        .map(|issue| Task {
            id: issue.id,
            title: issue.title,
            status: TaskStatus::from_raw(&issue.status),
            source: "beads".to_owned(),
            source_path: Some(root.join(".beads").display().to_string()),
            assignee: issue.owner,
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

    Ok(SourceLoadResult { tasks, warnings })
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
    updated_at: Option<String>,
    dependencies: Option<Vec<BeadDependency>>,
}

#[derive(Debug, Deserialize)]
struct BeadDependency {
    depends_on_id: String,
}
