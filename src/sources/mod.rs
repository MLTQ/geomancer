pub mod beads;
pub mod markdown;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::model::{LoadedSource, Task, TaskSnapshot};

pub fn load_repository(root: &Path) -> Result<TaskSnapshot, String> {
    let canonical_root = root
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(root));

    if !canonical_root.exists() {
        return Err(format!("{} does not exist", canonical_root.display()));
    }

    let mut snapshot = TaskSnapshot::empty(canonical_root.clone());
    let mut tasks = Vec::new();
    let mut warnings = Vec::new();
    let mut sources = Vec::new();

    if beads::detect(&canonical_root) {
        match beads::load(&canonical_root) {
            Ok(result) => {
                if !result.tasks.is_empty() {
                    sources.push(LoadedSource {
                        name: "beads".to_owned(),
                        task_count: result.tasks.len(),
                        detail: "Loaded through `bd list --json`".to_owned(),
                    });
                }
                warnings.extend(result.warnings);
                tasks.extend(result.tasks);
            }
            Err(error) => warnings.push(format!("beads: {error}")),
        }
    }

    match markdown::load(&canonical_root) {
        Ok(result) => {
            if !result.tasks.is_empty() {
                sources.push(LoadedSource {
                    name: "markdown".to_owned(),
                    task_count: result.tasks.len(),
                    detail: "Checklist items from repo markdown".to_owned(),
                });
            }
            warnings.extend(result.warnings);
            tasks.extend(result.tasks);
        }
        Err(error) => warnings.push(format!("markdown: {error}")),
    }

    tasks.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.title.cmp(&right.title))
    });
    populate_dependents(&mut tasks);
    deduplicate_warnings(&mut warnings);

    snapshot.tasks = tasks;
    snapshot.sources = sources;
    snapshot.warnings = warnings;
    Ok(snapshot)
}

fn populate_dependents(tasks: &mut [Task]) {
    let index: HashMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(position, task)| (task.id.clone(), position))
        .collect();
    let mut dependents: Vec<Vec<String>> = vec![Vec::new(); tasks.len()];

    for task in tasks.iter() {
        for dependency in &task.dependency_ids {
            if let Some(&dep_index) = index.get(dependency) {
                dependents[dep_index].push(task.id.clone());
            }
        }
    }

    for (task, outgoing) in tasks.iter_mut().zip(dependents) {
        task.dependent_ids = outgoing;
    }
}

fn deduplicate_warnings(warnings: &mut Vec<String>) {
    let mut seen = HashSet::new();
    warnings.retain(|warning| seen.insert(warning.clone()));
}

#[derive(Default)]
pub struct SourceLoadResult {
    pub tasks: Vec<Task>,
    pub warnings: Vec<String>,
}
