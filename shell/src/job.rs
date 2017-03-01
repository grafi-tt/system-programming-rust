use libc::pid_t;
use nix;
use nix::unistd;
use nix::sys::wait::WaitStatus;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum State { Active, Stopped, Terminated }

trait WaitStatusExt {
	fn get_pid(self) -> Option<pid_t>;
	fn state(self) -> State;
}

impl WaitStatusExt for WaitStatus {
	fn get_pid(self) -> Option<pid_t> {
		match self {
			WaitStatus::Exited(pid, ..) => Some(pid),
			WaitStatus::Signaled(pid, ..) => Some(pid),
			WaitStatus::Stopped(pid, ..) => Some(pid),
			#[cfg(any(target_os = "linux", target_os = "android"))]
			WaitStatus::PtraceEvent(pid, ..) => Some(pid),
			WaitStatus::Continued(pid) => Some(pid),
			WaitStatus::StillAlive => None,
		}
	}
	fn state(self) -> State {
		match self {
			WaitStatus::Exited(pid, ..) => State::Terminated,
			WaitStatus::Signaled(pid, ..) => State::Terminated,
			WaitStatus::Stopped(pid, ..) => State::Stopped,
			#[cfg(any(target_os = "linux", target_os = "android"))]
			WaitStatus::PtraceEvent(pid, ..) => State::Stopped,
			WaitStatus::Continued(pid) => State::Active,
			WaitStatus::StillAlive => State::Active,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Proccess {
	pub pid: pid_t,
	pub status: WaitStatus,
}

#[derive(Debug)]
pub struct Job {
	pub gid: pid_t,
	pub proccesses: Vec<Proccess>,
}

impl Job {
	pub fn state(&self) -> State {
		self.proccesses.iter().map(|pr| pr.status.state()).min().unwrap()
	}
}

#[derive(Debug)]
pub struct JobBuilder {
	imp: Job,
}

impl JobBuilder {
	pub fn new(size_hint: usize) -> JobBuilder {
		JobBuilder {
			imp: Job { gid: 0, proccesses: Vec::with_capacity(size_hint) }
		}
	}

	pub fn push_fork(&mut self) -> nix::Result<unistd::ForkResult> {
		let job = self.imp;

		let r = unistd::fork()?;
		match r {
			unistd::ForkResult::Parent{ child: pid } => {
				if job.gid == 0 {
					unistd::setpgid(pid, pid);
					job.gid = pid;
				} else {
					unistd::setpgid(pid, job.gid);
				}
				job.proccesses.push(Proccess { pid: pid, status: WaitStatus::StillAlive });
			},
			unistd::ForkResult::Child => {
				if job.gid == 0 {
					unistd::setpgid(0, 0);
				} else {
					unistd::setpgid(0, job.gid);
				}
			},
		}
		Ok(r)
	}

	pub fn build(self) -> Job {
		assert!(self.imp.proccesses.len() != 0);
		self.imp.proccesses.reverse();
		self.imp
	}
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct JobEvent {
	job_idx: usize,
	status: WaitStatus,
}

#[derive(Debug)]
pub struct JobSet {
	jobs: Vec<Option<Job>>,
	events: Vec<JobEvent>,
}

impl JobSet {
	fn update_job_set(&mut self, status: WaitStatus) -> JobDescriptor {
		let pid = status.get_pid().expect("wait returned 0");
		let events = self.events;
		for (i, &mut job) in self.jobs.iter_mut().enumerate() {
			if let Some(job) = job {
				let state = job.state();
				if let Some(pr) = job.proccesses.iter_mut().find(|pr| pr.pid == pid) {
					pr.status = status;
				} else {
					continue;
				}
				if state != job.state() {
					let rightmost_pr = job.proccesses.iter().rev().find(|pr| pr.status.state() == state).unwrap();
					events.push(JobEvent { job_idx: i, status: rightmost_pr.status });
				}
				return JobDescriptor { job_idx: i, job_set: self };
			}
		}
		panic!("unknown pid {}", pid)
	}

	pub fn push(&mut self, job: Job) -> JobDescriptor {
		let job_idx = {
			let jobs = self.jobs;
			if let Some((i, space)) = jobs.iter_mut().enumerate().find(|&(_, o)| o.is_none()) {
				*space = Some(job);
				i
			} else {
				let len = jobs.len();
				jobs.push(Some(job));
				len
			}
		};
		JobDescriptor { job_idx: job_idx, job_set: self }
	}
}

#[derive(Debug)]
pub struct JobDescriptor<'a> {
	job_idx: usize,
	job_set: &'a mut JobSet,
}

impl<'a> Drop for JobDescriptor<'a> {
	fn drop(&mut self) {
		if self.job().state() == State::Terminated {
			let jobs = self.job_set.jobs;
			jobs[self.job_idx] = None;
			let len = jobs.iter().enumerate().rev().find(|&(i, pr)| pr.is_some()).map_or(0, |(i, _)| i + 1);
			jobs.truncate(len);
		}
	}
}

impl<'a> JobDescriptor<'a> {
	pub fn job(&self) -> &Job {
		&self.job_set.jobs[self.job_idx].unwrap()
	}

	pub fn wait(self) -> JobDescriptor<'a> {
		let wait_job_idx = self.job_idx;
		let wait_state = self.job().state();
		let job_set = self.job_set;
		drop(self);
		loop {
			let status = nix::sys::wait::wait().expect("wait errored");
			let job_desc = job_set.update_job_set(status);
			let job = job_desc.job();
			if job_desc.job_idx == wait_job_idx && job.state() != wait_state {
				if let (State::Terminated, WaitStatus::Exited(..)) = (job.state(), job.proccesses.last().unwrap().status) {
					job_desc.job_set.events.pop();
				}
				return job_desc;
			}
		}
		unreachable!()
	}
}
