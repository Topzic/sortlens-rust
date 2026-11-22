//! The activity stack: what the engine is doing, and what it just failed to do.
//!
//! One list, newest first. Every long operation creates a job when it starts and updates it when it
//! finishes, so the Activity panel is a view of the engine rather than a separate logging system.

use std::time::{Duration, Instant};

/// What kind of work a job is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Walking a folder and updating the catalog.
    Scan,
    /// Decoding thumbnails.
    Preview,
    /// Writing files.
    Export,
    /// Writing XMP sidecars beside photos.
    Sidecar,
    /// Hashing previews to find near-duplicates.
    Hash,
    /// Counting the library for the insights view.
    Insights,
}

impl JobKind {
    /// Label for the activity row.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Scan => "Scan",
            Self::Preview => "Previews",
            Self::Export => "Export",
            Self::Sidecar => "Sidecars",
            Self::Hash => "Similar",
            Self::Insights => "Insights",
        }
    }
}

/// Where a job is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Still working.
    Running,
    /// Finished successfully.
    Done,
    /// Stopped because the user asked.
    Cancelled,
    /// Stopped because something went wrong.
    Failed,
}

impl JobStatus {
    /// Whether the job is still running.
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Short label for the row.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// One operation the engine ran or is running.
#[derive(Debug, Clone)]
pub struct Job {
    /// Monotonic id, unique within a session.
    pub id: u64,
    /// What kind of work it is.
    pub kind: JobKind,
    /// Human label: the folder name for a scan.
    pub label: String,
    /// Current state.
    pub status: JobStatus,
    /// Files accepted so far.
    pub files: usize,
    /// Folders visited so far.
    pub dirs: usize,
    /// Files that could not be read, when the job finished.
    pub unreadable: usize,
    /// The failure, when the job failed.
    pub error: Option<String>,
    /// When it started.
    pub started: Instant,
    /// When it stopped, if it has.
    pub finished: Option<Instant>,
}

impl Job {
    /// A job that has just started.
    pub fn running(id: u64, kind: JobKind, label: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            label: label.into(),
            status: JobStatus::Running,
            files: 0,
            dirs: 0,
            unreadable: 0,
            error: None,
            started: Instant::now(),
            finished: None,
        }
    }

    /// How long it ran, or has been running.
    pub fn elapsed(&self) -> Duration {
        self.finished.unwrap_or_else(Instant::now).saturating_duration_since(self.started)
    }

    /// One-line summary for the activity row.
    pub fn detail(&self) -> String {
        match self.status {
            JobStatus::Running => format!("{} files · {} folders", self.files, self.dirs),
            JobStatus::Done => {
                let mut text = format!("{} files · {} folders", self.files, self.dirs);
                if self.unreadable > 0 {
                    text.push_str(&format!(" · {} unreadable", self.unreadable));
                }
                text
            }
            JobStatus::Cancelled => "cancelled before committing".to_string(),
            JobStatus::Failed => self.error.clone().unwrap_or_else(|| "failed".to_string()),
        }
    }

    /// Mark the job finished.
    pub fn finish(&mut self, status: JobStatus) {
        self.status = status;
        self.finished = Some(Instant::now());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_running_job_reports_its_counters() {
        let mut job = Job::running(1, JobKind::Scan, "Shoot");
        job.files = 12;
        job.dirs = 3;
        assert!(job.status.is_running());
        assert_eq!(job.detail(), "12 files · 3 folders");
    }

    #[test]
    fn finishing_stops_the_clock() {
        let mut job = Job::running(1, JobKind::Scan, "Shoot");
        job.finish(JobStatus::Done);
        assert!(job.finished.is_some());
        assert_eq!(job.status.label(), "done");
    }

    #[test]
    fn a_failed_job_shows_its_error_instead_of_counts() {
        let mut job = Job::running(2, JobKind::Scan, "Shoot");
        job.error = Some("permission denied".into());
        job.finish(JobStatus::Failed);
        assert_eq!(job.detail(), "permission denied");
    }

    #[test]
    fn a_cancelled_job_says_nothing_was_committed() {
        let mut job = Job::running(3, JobKind::Scan, "Shoot");
        job.finish(JobStatus::Cancelled);
        assert_eq!(job.detail(), "cancelled before committing");
    }
}
