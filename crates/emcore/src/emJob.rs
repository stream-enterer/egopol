use crate::emCore::emSignal::SignalId;
use super::emScheduler::EngineScheduler;

/// The state of a job in the queue.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JobState {
    /// Not currently in any queue.
    NotEnqueued,
    /// Waiting to be started (sorted by priority).
    Waiting,
    /// Currently running.
    Running,
    /// Was aborted before completion.
    Aborted,
    /// Completed successfully.
    Success,
    /// Completed with an error.
    Error,
}

/// A unit of deferred work managed by a `emJobQueue`.
///
/// This is the Rust equivalent of the C++ `emJob`. Jobs have a priority that
/// determines their execution order in the waiting list, and a state that
/// tracks their lifecycle.
pub struct emJob {
    priority: f64,
    state: JobState,
    error_text: String,
    /// Signal fired on state transitions.
    state_signal: SignalId,
    /// Index into the queue's waiting or running list, or `None` if not enqueued.
    queue_slot: Option<QueueSlot>,
}

#[derive(Copy, Clone, Debug)]
enum QueueSlot {
    Waiting(usize),
    Running(usize),
}

/// Opaque handle to a job within a `emJobQueue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct JobId(usize);

impl emJob {
    /// Create a new job with the given priority. A signal is allocated from the
    /// scheduler for state-change notifications.
    pub fn new(priority: f64, scheduler: &mut EngineScheduler) -> Self {
        let state_signal = scheduler.create_signal();
        Self {
            priority,
            state: JobState::NotEnqueued,
            error_text: String::new(),
            state_signal,
            queue_slot: None,
        }
    }

    /// Get the job's priority.
    pub fn GetPriority(&self) -> f64 {
        self.priority
    }

    /// Get the current state.
    pub fn GetState(&self) -> JobState {
        self.state
    }

    /// Get the signal that is fired on state transitions.
    pub fn GetStateSignal(&self) -> SignalId {
        self.state_signal
    }

    /// Get the error text (only meaningful when state is `Error`).
    pub fn GetErrorText(&self) -> &str {
        &self.error_text
    }

    /// Remove the state signal from the scheduler. Call before dropping.
    pub fn remove_signal(self, scheduler: &mut EngineScheduler) {
        scheduler.remove_signal(self.state_signal);
    }
}

impl Drop for emJob {
    fn drop(&mut self) {
        debug_assert!(
            self.queue_slot.is_none(),
            "Job destructed while still referenced by a JobQueue (state: {:?})",
            self.state,
        );
    }
}

/// A queue that manages the lifecycle of jobs: enqueuing, priority-sorted
/// waiting, starting, and completion.
///
/// This is the Rust equivalent of the C++ `emJobQueue`. Jobs in the waiting
/// list are sorted by priority (highest first). The queue maintains separate
/// lists for waiting and running jobs.
pub struct emJobQueue {
    /// Jobs stored by index. Indices are stable (we never remove, just mark as empty).
    jobs: Vec<Option<emJob>>,
    /// Indices into `jobs` for waiting jobs, sorted by priority (highest first).
    waiting: Vec<usize>,
    /// Indices into `jobs` for running jobs, in insertion order.
    running: Vec<usize>,
    /// Whether the waiting list needs re-sorting.
    sorting_invalid: bool,
}

impl emJobQueue {
    /// Create a new empty job queue.
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            waiting: Vec::new(),
            running: Vec::new(),
            sorting_invalid: false,
        }
    }

    /// Add a job to the queue and return its handle. The job transitions to
    /// `Waiting` state and its state signal is fired.
    pub fn EnqueueJob(&mut self, mut job: emJob, scheduler: &mut EngineScheduler) -> JobId {
        let idx = self.jobs.len();
        let id = JobId(idx);

        job.state = JobState::Waiting;
        job.error_text.clear();
        scheduler.fire(job.state_signal);

        let wait_pos = self.waiting.len();
        self.waiting.push(idx);
        job.queue_slot = Some(QueueSlot::Waiting(wait_pos));

        self.jobs.push(Some(job));
        self.sorting_invalid = true;
        id
    }

    /// Whether the queue has no waiting or running jobs.
    pub fn IsEmpty(&self) -> bool {
        self.waiting.is_empty() && self.running.is_empty()
    }

    /// Get a reference to a job by its handle.
    pub fn GetRec(&self, id: JobId) -> Option<&emJob> {
        self.jobs.get(id.0).and_then(|slot| slot.as_ref())
    }

    /// Get a mutable reference to a job by its handle.
    pub fn get_mut(&mut self, id: JobId) -> Option<&mut emJob> {
        self.jobs.get_mut(id.0).and_then(|slot| slot.as_mut())
    }

    /// Set a job's priority. If the job is waiting, the sorting is invalidated.
    pub fn SetPriority(&mut self, id: JobId, priority: f64) {
        if let Some(job) = self.get_mut(id) {
            if (job.priority - priority).abs() > f64::EPSILON {
                job.priority = priority;
                if job.state == JobState::Waiting {
                    self.sorting_invalid = true;
                }
            }
        }
    }

    /// Get the first (highest-priority) waiting job's ID, or `None`.
    pub fn GetFirstWaitingJob(&mut self) -> Option<JobId> {
        self.UpdateSortingOfWaitingJobs();
        self.waiting.first().map(|&idx| JobId(idx))
    }

    /// Get the first running job's ID, or `None`.
    pub fn GetFirstRunningJob(&self) -> Option<JobId> {
        self.running.first().map(|&idx| JobId(idx))
    }

    /// Mark the waiting list as needing re-sort.
    pub fn InvalidateSortingOfWaitingJobs(&mut self) {
        self.sorting_invalid = true;
    }

    /// Re-sort the waiting list by priority (highest first) if needed.
    pub fn UpdateSortingOfWaitingJobs(&mut self) {
        if !self.sorting_invalid {
            return;
        }
        let jobs = &self.jobs;
        self.waiting.sort_by(|&a, &b| {
            let pa = jobs[a].as_ref().map_or(0.0, |j| j.priority);
            let pb = jobs[b].as_ref().map_or(0.0, |j| j.priority);
            // Descending: higher priority first.
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Update queue_slot indices after sort.
        for (pos, &idx) in self.waiting.iter().enumerate() {
            if let Some(job) = self.jobs[idx].as_mut() {
                job.queue_slot = Some(QueueSlot::Waiting(pos));
            }
        }
        self.sorting_invalid = false;
    }

    /// Start the next highest-priority waiting job. Returns its ID, or `None`
    /// if no jobs are waiting.
    pub fn StartNextJob(&mut self, scheduler: &mut EngineScheduler) -> Option<JobId> {
        let id = self.GetFirstWaitingJob()?;
        self.StartJob(id, scheduler);
        Some(id)
    }

    /// Move a job from waiting to running.
    pub fn StartJob(&mut self, id: JobId, scheduler: &mut EngineScheduler) {
        let Some(job) = self.jobs.get(id.0).and_then(|s| s.as_ref()) else {
            return;
        };
        if job.state == JobState::Running {
            return;
        }

        // Remove from waiting list if present.
        self.remove_from_lists(id);

        // Add to running list.
        let run_pos = self.running.len();
        self.running.push(id.0);

        let job = self.jobs[id.0].as_mut().expect("job exists");
        job.queue_slot = Some(QueueSlot::Running(run_pos));
        job.state = JobState::Running;
        scheduler.fire(job.state_signal);
    }

    /// Abort a job, removing it from the queue.
    pub fn AbortJob(&mut self, id: JobId, scheduler: &mut EngineScheduler) {
        self.finish_job(id, JobState::Aborted, String::new(), scheduler);
    }

    /// Mark a job as successfully completed, removing it from the queue.
    pub fn SucceedJob(&mut self, id: JobId, scheduler: &mut EngineScheduler) {
        self.finish_job(id, JobState::Success, String::new(), scheduler);
    }

    /// Mark a job as failed with an error message, removing it from the queue.
    pub fn FailJob(&mut self, id: JobId, error_text: String, scheduler: &mut EngineScheduler) {
        self.finish_job(id, JobState::Error, error_text, scheduler);
    }

    /// Fail all running jobs with the given error.
    pub fn FailAllRunningJobs(&mut self, error_text: &str, scheduler: &mut EngineScheduler) {
        let running_ids: Vec<JobId> = self.running.iter().map(|&idx| JobId(idx)).collect();
        for id in running_ids {
            self.finish_job(id, JobState::Error, error_text.to_string(), scheduler);
        }
    }

    /// Fail all jobs (running and waiting) with the given error.
    pub fn FailAllJobs(&mut self, error_text: &str, scheduler: &mut EngineScheduler) {
        self.FailAllRunningJobs(error_text, scheduler);
        let waiting_ids: Vec<JobId> = self.waiting.iter().map(|&idx| JobId(idx)).collect();
        for id in waiting_ids {
            self.finish_job(id, JobState::Error, error_text.to_string(), scheduler);
        }
    }

    /// Remove all jobs. Running and waiting jobs are aborted.
    pub fn Clear(&mut self, scheduler: &mut EngineScheduler) {
        let running_ids: Vec<JobId> = self.running.iter().map(|&idx| JobId(idx)).collect();
        for id in running_ids {
            self.AbortJob(id, scheduler);
        }
        let waiting_ids: Vec<JobId> = self.waiting.iter().map(|&idx| JobId(idx)).collect();
        for id in waiting_ids {
            self.AbortJob(id, scheduler);
        }
    }

    /// Iterate over waiting job IDs.
    pub fn waiting_jobs(&mut self) -> Vec<JobId> {
        self.UpdateSortingOfWaitingJobs();
        self.waiting.iter().map(|&idx| JobId(idx)).collect()
    }

    /// Iterate over running job IDs.
    pub fn running_jobs(&self) -> Vec<JobId> {
        self.running.iter().map(|&idx| JobId(idx)).collect()
    }

    // ── Internal helpers ─────────────────────────────────────────────

    fn finish_job(
        &mut self,
        id: JobId,
        new_state: JobState,
        error_text: String,
        scheduler: &mut EngineScheduler,
    ) {
        let Some(job) = self.jobs.get_mut(id.0).and_then(|s| s.as_mut()) else {
            return;
        };

        job.state = new_state;
        job.error_text = error_text;
        scheduler.fire(job.state_signal);

        self.remove_from_lists(id);
        if let Some(job) = self.jobs[id.0].as_mut() {
            job.queue_slot = None;
        }
    }

    fn remove_from_lists(&mut self, id: JobId) {
        let Some(job) = self.jobs.get(id.0).and_then(|s| s.as_ref()) else {
            return;
        };
        let Some(slot) = job.queue_slot else {
            return;
        };

        match slot {
            QueueSlot::Waiting(pos) => {
                if pos < self.waiting.len() && self.waiting[pos] == id.0 {
                    self.waiting.remove(pos);
                    // Update queue_slot for shifted entries.
                    for (new_pos, &idx) in self.waiting.iter().enumerate().skip(pos) {
                        if let Some(j) = self.jobs[idx].as_mut() {
                            j.queue_slot = Some(QueueSlot::Waiting(new_pos));
                        }
                    }
                }
            }
            QueueSlot::Running(pos) => {
                if pos < self.running.len() && self.running[pos] == id.0 {
                    self.running.remove(pos);
                    // Update queue_slot for shifted entries.
                    for (new_pos, &idx) in self.running.iter().enumerate().skip(pos) {
                        if let Some(j) = self.jobs[idx].as_mut() {
                            j.queue_slot = Some(QueueSlot::Running(new_pos));
                        }
                    }
                }
            }
        }
    }
}

impl Default for emJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_start() {
        let mut sched = EngineScheduler::new();
        let mut queue = emJobQueue::new();

        let job = emJob::new(1.0, &mut sched);
        let sig = job.GetStateSignal();
        let id = queue.EnqueueJob(job, &mut sched);

        assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Waiting);
        assert!(!queue.IsEmpty());

        queue.StartJob(id, &mut sched);
        assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Running);

        queue.SucceedJob(id, &mut sched);
        assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Success);
        assert!(queue.IsEmpty());

        sched.remove_signal(sig);
    }

    #[test]
    fn priority_ordering() {
        let mut sched = EngineScheduler::new();
        let mut queue = emJobQueue::new();

        let job_low = emJob::new(1.0, &mut sched);
        let sig_low = job_low.GetStateSignal();
        let id_low = queue.EnqueueJob(job_low, &mut sched);

        let job_high = emJob::new(10.0, &mut sched);
        let sig_high = job_high.GetStateSignal();
        let id_high = queue.EnqueueJob(job_high, &mut sched);

        // start_next should pick the highest priority job.
        let started = queue.StartNextJob(&mut sched);
        assert_eq!(started, Some(id_high));
        assert_eq!(queue.GetRec(id_high).unwrap().GetState(), JobState::Running);
        assert_eq!(queue.GetRec(id_low).unwrap().GetState(), JobState::Waiting);

        queue.AbortJob(id_high, &mut sched);
        queue.AbortJob(id_low, &mut sched);
        sched.remove_signal(sig_low);
        sched.remove_signal(sig_high);
    }

    #[test]
    fn fail_job_records_error() {
        let mut sched = EngineScheduler::new();
        let mut queue = emJobQueue::new();

        let job = emJob::new(1.0, &mut sched);
        let sig = job.GetStateSignal();
        let id = queue.EnqueueJob(job, &mut sched);
        queue.StartJob(id, &mut sched);

        queue.FailJob(id, "disk full".to_string(), &mut sched);
        let j = queue.GetRec(id).unwrap();
        assert_eq!(j.GetState(), JobState::Error);
        assert_eq!(j.GetErrorText(), "disk full");

        sched.remove_signal(sig);
    }

    #[test]
    fn fail_all_jobs() {
        let mut sched = EngineScheduler::new();
        let mut queue = emJobQueue::new();

        let job1 = emJob::new(1.0, &mut sched);
        let sig1 = job1.GetStateSignal();
        let id1 = queue.EnqueueJob(job1, &mut sched);
        queue.StartJob(id1, &mut sched);

        let job2 = emJob::new(2.0, &mut sched);
        let sig2 = job2.GetStateSignal();
        let id2 = queue.EnqueueJob(job2, &mut sched);

        queue.FailAllJobs("shutdown", &mut sched);

        assert_eq!(queue.GetRec(id1).unwrap().GetState(), JobState::Error);
        assert_eq!(queue.GetRec(id2).unwrap().GetState(), JobState::Error);
        assert!(queue.IsEmpty());

        sched.remove_signal(sig1);
        sched.remove_signal(sig2);
    }

    #[test]
    fn AbortJob() {
        let mut sched = EngineScheduler::new();
        let mut queue = emJobQueue::new();

        let job = emJob::new(1.0, &mut sched);
        let sig = job.GetStateSignal();
        let id = queue.EnqueueJob(job, &mut sched);
        queue.AbortJob(id, &mut sched);

        assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Aborted);
        assert!(queue.IsEmpty());

        sched.remove_signal(sig);
    }
}
