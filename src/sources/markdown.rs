use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::model::{Task, TaskStatus};

use super::SourceLoadResult;

pub fn load(root: &Path) -> Result<SourceLoadResult, String> {
    let mut markdown_files = Vec::new();
    collect_markdown_files(root, &mut markdown_files, 0)?;

    let mut tasks = Vec::new();

    for file in markdown_files {
        let content = match fs::read_to_string(&file) {
            Ok(content) => content,
            Err(_) => continue,
        };

        for (line_index, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let status = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
                Some((TaskStatus::Open, rest))
            } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
                Some((TaskStatus::Done, rest))
            } else if let Some(rest) = trimmed.strip_prefix("- [X] ") {
                Some((TaskStatus::Done, rest))
            } else {
                None
            };

            let Some((status, title)) = status else {
                continue;
            };

            tasks.push(Task {
                id: format!("md:{}:{}", file.display(), line_index + 1),
                title: title.trim().to_owned(),
                status,
                source: "markdown".to_owned(),
                source_path: Some(file.display().to_string()),
                assignee: None,
                updated_at: None,
                dependency_ids: Vec::new(),
                dependent_ids: Vec::new(),
                url: None,
            });
        }
    }

    Ok(SourceLoadResult {
        tasks,
        warnings: Vec::new(),
    })
}

fn collect_markdown_files(root: &Path, files: &mut Vec<PathBuf>, depth: usize) -> Result<(), String> {
    if depth > 6 {
        return Ok(());
    }

    let entries = fs::read_dir(root)
        .map_err(|error| format!("failed to read {}: {error}", root.display()))?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            if matches!(
                file_name.as_ref(),
                ".git" | ".beads" | "target" | "node_modules" | ".idea" | ".zed"
            ) {
                continue;
            }
            collect_markdown_files(&path, files, depth + 1)?;
            continue;
        }

        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| matches!(value, "md" | "markdown"))
        {
            files.push(path);
        }
    }

    Ok(())
}
