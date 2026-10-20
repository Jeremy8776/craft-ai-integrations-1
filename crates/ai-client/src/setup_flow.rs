//! Reusable setup state for any host UI. Approval and worker answers belong to one exact plan.
use crate::{CheckedNative, NativeSetupPlan};
use craft_ai_core::{AiError, ErrorKind};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupTask {
    CheckExisting,
    Download,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetupTicket(u64);

#[derive(Clone, Debug)]
pub struct SetupRequest {
    ticket: SetupTicket,
    plan: NativeSetupPlan,
    task: SetupTask,
    approved: bool,
    cancelled: Arc<AtomicBool>,
}
impl SetupRequest {
    pub fn ticket(&self) -> SetupTicket {
        self.ticket
    }
    pub fn plan(&self) -> &NativeSetupPlan {
        &self.plan
    }
    pub fn task(&self) -> SetupTask {
        self.task
    }
    /// Also available to a host shutting down a worker without a mounted setup window.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn approved(&self) -> bool {
        self.approved
    }
    /// Carry this out on a worker. Tokens are borrowed for this call and never stored in a request.
    pub fn run(
        &self,
        downloader: &craft_ai_comfy_env::Downloader,
        token: Option<&craft_ai_core::SecretString>,
        progress: &mut dyn FnMut(usize, craft_ai_comfy_env::Progress),
    ) -> Result<CheckedNative, AiError> {
        let cancel = self.cancelled.as_ref();
        match self.task {
            SetupTask::CheckExisting => self.plan.check_existing(cancel, progress),
            SetupTask::Download => self.plan.download(crate::NativeInstall {
                downloader,
                token,
                licences_accepted: self.approved,
                cancel,
                progress,
            }),
        }
    }
}

pub struct SetupSession {
    plan: NativeSetupPlan,
    accepted: bool,
    next: u64,
    active: Option<SetupRequest>,
    checked: Option<CheckedNative>,
    problem: Option<ErrorKind>,
    progress: Option<(usize, crate::SetupProgress)>,
}
impl SetupSession {
    pub fn new(plan: NativeSetupPlan) -> Self {
        Self {
            plan,
            accepted: false,
            next: 0,
            active: None,
            checked: None,
            problem: None,
            progress: None,
        }
    }
    pub fn plan(&self) -> &NativeSetupPlan {
        &self.plan
    }
    pub fn accepted(&self) -> bool {
        self.accepted
    }
    pub fn accept_licences(&mut self, accepted: bool) {
        self.accepted = accepted;
    }
    pub fn busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn checked(&self) -> Option<&CheckedNative> {
        self.checked.as_ref()
    }
    pub fn progress(&self) -> Option<(usize, crate::SetupProgress)> {
        self.progress
    }
    pub fn report_progress(
        &mut self,
        ticket: SetupTicket,
        file: usize,
        progress: crate::SetupProgress,
    ) -> bool {
        if self
            .active
            .as_ref()
            .is_none_or(|active| active.ticket != ticket)
            || file >= self.plan.route().models.len()
        {
            return false;
        }
        self.progress = Some((file, progress));
        true
    }
    pub fn problem(&self) -> Option<ErrorKind> {
        self.problem
    }
    /// Return the old ticket so the host can cancel its worker. Stale answers cannot update this session.
    pub fn review(&mut self, plan: NativeSetupPlan) -> Option<SetupTicket> {
        if self.plan == plan {
            return None;
        }
        let previous = self.cancel();
        self.plan = plan;
        self.accepted = false;
        self.checked = None;
        self.problem = None;
        self.progress = None;
        previous
    }
    /// Consumes approval and returns a worker request, never performs I/O on the drawing thread.
    pub fn begin(&mut self, task: SetupTask) -> Option<SetupRequest> {
        if self.busy() || !self.plan.available() {
            return None;
        }
        match task {
            SetupTask::Download if !self.accepted || self.plan.destination().is_none() => {
                return None;
            }
            SetupTask::CheckExisting if self.plan.existing_directory().is_none() => return None,
            _ => {}
        }
        self.next = self.next.checked_add(1)?;
        let request = SetupRequest {
            ticket: SetupTicket(self.next),
            plan: self.plan.clone(),
            task,
            approved: self.accepted,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        self.accepted = false;
        self.checked = None;
        self.problem = None;
        self.progress = None;
        self.active = Some(request.clone());
        Some(request)
    }
    /// Accept a worker answer without trusting its raw error text.
    pub fn complete(
        &mut self,
        ticket: SetupTicket,
        result: Result<CheckedNative, AiError>,
    ) -> bool {
        if self
            .active
            .as_ref()
            .is_none_or(|active| active.ticket != ticket)
        {
            return false;
        }
        let Some(active) = self.active.take() else {
            return false;
        };
        self.progress = None;
        let directory = match active.task {
            SetupTask::CheckExisting => active.plan.existing_directory(),
            SetupTask::Download => active.plan.destination(),
        };
        let expected = directory
            .zip(active.plan.route().weights())
            .map(|(dir, file)| dir.join(&file.filename));
        match result {
            Ok(checked)
                if checked.feature() == active.plan.feature()
                    && checked.provider() == active.plan.route().provider
                    && expected.as_deref() == Some(checked.weights()) =>
            {
                self.checked = Some(checked)
            }
            Ok(_) => {
                self.checked = None;
                self.problem = Some(ErrorKind::InvalidRequest);
            }
            Err(error) => {
                self.checked = None;
                self.problem = Some(error.kind);
            }
        }
        true
    }
    pub fn cancel(&mut self) -> Option<SetupTicket> {
        self.progress = None;
        self.active.take().map(|request| {
            request.cancel();
            request.ticket
        })
    }
}
