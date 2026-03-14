use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Unknown(String),
}

impl TaskStatus {
    pub fn from_raw(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "open" | "todo" | "pending" => Self::Open,
            "in_progress" | "in-progress" | "doing" | "active" => Self::InProgress,
            "blocked" | "waiting" | "deferred" => Self::Blocked,
            "done" | "closed" | "complete" | "completed" | "resolved" => Self::Done,
            other => Self::Unknown(other.to_owned()),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Unknown(value) => value.as_str(),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub source: String,
    pub source_path: Option<String>,
    pub assignee: Option<String>,
    pub updated_at: Option<String>,
    pub dependency_ids: Vec<String>,
    pub dependent_ids: Vec<String>,
    pub url: Option<String>,
}

impl Task {
    pub fn is_done(&self) -> bool {
        self.status.is_done()
    }

    pub fn is_blocked(&self) -> bool {
        self.status.is_blocked()
            || self
                .dependency_ids
                .iter()
                .any(|dependency| !dependency.is_empty())
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoadedSource {
    pub name: String,
    pub task_count: usize,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct TaskSnapshot {
    pub root: PathBuf,
    pub tasks: Vec<Task>,
    pub sources: Vec<LoadedSource>,
    pub warnings: Vec<String>,
}

impl TaskSnapshot {
    pub fn empty(root: PathBuf) -> Self {
        Self {
            root,
            tasks: Vec::new(),
            sources: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn stats(&self) -> SnapshotStats {
        let mut stats = SnapshotStats::default();

        for task in &self.tasks {
            stats.total += 1;
            match task.status {
                TaskStatus::Done => stats.done += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Blocked => stats.blocked += 1,
                TaskStatus::Open | TaskStatus::Unknown(_) => stats.open += 1,
            }
        }

        stats
    }

    pub fn task_index(&self) -> HashMap<&str, usize> {
        self.tasks
            .iter()
            .enumerate()
            .map(|(index, task)| (task.id.as_str(), index))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotStats {
    pub total: usize,
    pub done: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub open: usize,
}

impl SnapshotStats {
    pub fn completion_ratio(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.done as f32 / self.total as f32
        }
    }
}
