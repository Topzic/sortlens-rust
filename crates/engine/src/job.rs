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
    /// Proposing keywords for the whole library.
    Suggest,
    /// Measuring how much fine detail each photograph has.
    Sharpness,
    /// Fetching a model into the store (SL-F02, opt-in).
    ModelInstall,
    /// Checking for an update (SL-I10, opt-in).
    UpdateCheck,
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
            Self::Suggest => "Suggestions",
            Self::Sharpness => "Sharpness",
            Self::ModelInstall => "Model",
            Self::UpdateCheck => "Update",
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

/// The piece a job's one-line detail is joined from.
///
/// The shell words these in the interface language and [`Job::detail`] joins the English from
/// them, so a translated activity row and the report the same library writes cannot describe
/// different work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobDetail {
    /// A running detail the counters cannot express — a download's bytes, for one.
    Progress(String),
    /// The counters while the job runs.
    Counts {
        /// Files accepted so far.
        files: usize,
        /// Folders visited so far.
        dirs: usize,
    },
    /// The counters after it finished, with how many files it could not read.
    Finished {
        /// Files accepted.
        files: usize,
        /// Folders visited.
        dirs: usize,
        /// Files that could not be read; `None` when every one was.
        unreadable: Option<usize>,
    },
    /// Stopped because the user asked.
    Cancelled,
    /// Stopped because something went wrong; the error when there is one.
    Failed(Option<String>),
}

impl JobDetail {
    /// The English line, byte for byte what the row said before it took a piece.
    #[must_use]
    pub fn english(&self) -> String {
        match self {
            Self::Progress(progress) => progress.clone(),
            Self::Counts { files, dirs } => format!("{files} files · {dirs} folders"),
            Self::Finished {
                files,
                dirs,
                unreadable: None,
            } => format!("{files} files · {dirs} folders"),
            Self::Finished {
                files,
                dirs,
                unreadable: Some(unreadable),
            } => {
                format!("{files} files · {dirs} folders · {unreadable} unreadable")
            }
            Self::Cancelled => "cancelled before committing".to_string(),
            Self::Failed(Some(error)) => error.clone(),
            Self::Failed(None) => "failed".to_string(),
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
    /// A running detail the counters cannot express — a download's bytes, for one.
    pub progress: Option<String>,
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
            progress: None,
            started: Instant::now(),
            finished: None,
        }
    }

    /// How long it ran, or has been running.
    pub fn elapsed(&self) -> Duration {
        self.finished.unwrap_or_else(Instant::now).saturating_duration_since(self.started)
    }

    /// The piece the row is worded from: the shell translates it, and [`Self::detail`] is its
    /// English.
    #[must_use]
    pub fn detail_kind(&self) -> JobDetail {
        if let Some(progress) = &self.progress {
            return JobDetail::Progress(progress.clone());
        }
        match self.status {
            JobStatus::Running => JobDetail::Counts {
                files: self.files,
                dirs: self.dirs,
            },
            JobStatus::Done => JobDetail::Finished {
                files: self.files,
                dirs: self.dirs,
                unreadable: (self.unreadable > 0).then_some(self.unreadable),
            },
            JobStatus::Cancelled => JobDetail::Cancelled,
            JobStatus::Failed => JobDetail::Failed(self.error.clone()),
        }
    }

    /// One-line summary for the activity row.
    pub fn detail(&self) -> String {
        self.detail_kind().english()
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
    fn the_detail_english_is_joined_from_its_piece() {
        let mut job = Job::running(4, JobKind::Scan, "Shoot");
        job.files = 12;
        job.dirs = 3;
        assert_eq!(job.detail_kind(), JobDetail::Counts { files: 12, dirs: 3 });
        assert_eq!(job.detail(), job.detail_kind().english());
        assert_eq!(job.detail(), "12 files · 3 folders");

        job.finish(JobStatus::Done);
        assert_eq!(
            job.detail_kind(),
            JobDetail::Finished {
                files: 12,
                dirs: 3,
                unreadable: None
            }
        );
        assert_eq!(job.detail(), "12 files · 3 folders");

        job.unreadable = 2;
        assert_eq!(
            job.detail_kind(),
            JobDetail::Finished {
                files: 12,
                dirs: 3,
                unreadable: Some(2)
            }
        );
        assert_eq!(job.detail(), "12 files · 3 folders · 2 unreadable");

        job.progress = Some("1.5 MB of 4.0 MB".to_string());
        assert_eq!(job.detail_kind(), JobDetail::Progress("1.5 MB of 4.0 MB".to_string()));
        assert_eq!(job.detail(), "1.5 MB of 4.0 MB");

        let mut cancelled = Job::running(5, JobKind::Export, "Out");
        cancelled.finish(JobStatus::Cancelled);
        assert_eq!(cancelled.detail_kind(), JobDetail::Cancelled);
        assert_eq!(cancelled.detail(), "cancelled before committing");

        let mut failed = Job::running(6, JobKind::Scan, "Shoot");
        failed.finish(JobStatus::Failed);
        assert_eq!(failed.detail_kind(), JobDetail::Failed(None));
        assert_eq!(failed.detail(), "failed");
        failed.error = Some("permission denied".into());
        assert_eq!(failed.detail_kind(), JobDetail::Failed(Some("permission denied".into())));
        assert_eq!(failed.detail(), "permission denied");
    }

    #[test]
    fn a_cancelled_job_says_nothing_was_committed() {
        let mut job = Job::running(3, JobKind::Scan, "Shoot");
        job.finish(JobStatus::Cancelled);
        assert_eq!(job.detail(), "cancelled before committing");
    }
}
