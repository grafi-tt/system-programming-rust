use libc::pid_t;
use nix;
use nix::unistd;
use nix::sys::wait::WaitStatus;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum State { Active, Stopped, Terminated }

pub trait WaitStatusExt {
	fn get_pid(self) -> Option<pid_t>;
	fn state(self) -> State;
	fn code(self) -> u8;
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
			WaitStatus::Exited(..) => State::Terminated,
			WaitStatus::Signaled(..) => State::Terminated,
			WaitStatus::Stopped(..) => State::Stopped,
			#[cfg(any(target_os = "linux", target_os = "android"))]
			WaitStatus::PtraceEvent(..) => State::Stopped,
			WaitStatus::Continued(..) => State::Active,
			WaitStatus::StillAlive => State::Active,
		}
	}
	fn code(self) -> u8 {
		match self {
			WaitStatus::Exited(_, code) => code as u8,
			WaitStatus::Signaled(_, sig, _) => sig as u8,
			_ => 0,
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
		let job = &mut self.imp;

		let r = unistd::fork()?;
		match r {
			unistd::ForkResult::Parent{ child: pid } => {
				if job.gid == 0 {
					let _ = unistd::setpgid(pid, pid);
					job.gid = pid;
				} else {
					let _ = unistd::setpgid(pid, job.gid);
				}
				job.proccesses.push(Proccess { pid: pid, status: WaitStatus::StillAlive });
			},
			unistd::ForkResult::Child => {
				if job.gid == 0 {
					let _ = unistd::setpgid(0, 0);
				} else {
					let _ = unistd::setpgid(0, job.gid);
				}
			},
		}
		Ok(r)
	}

	pub fn build(mut self) -> Job {
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
	fn update_job_set(&mut self, status: WaitStatus) -> usize {
		let pid = status.get_pid().expect("wait returned 0");
		let job_idx = {
			let mut go = || {
				for (i, job) in self.jobs.iter_mut().enumerate() {
					if let Some(ref mut job) = *job {
						let state = job.state();
						if let Some(pr) = job.proccesses.iter_mut().find(|pr| pr.pid == pid) {
							pr.status = status;
						} else {
							continue;
						}
						if state != job.state() {
							let rightmost_pr = job.proccesses.iter().rev().find(|pr| pr.status.state() == state).unwrap();
							self.events.push(JobEvent { job_idx: i, status: rightmost_pr.status });
						}
						return i;
					}
				}
				panic!("unknown pid {}", pid)
			};
			go()
		};
		job_idx
	}

	pub fn push(&mut self, job: Job) -> JobDescriptor {
		let job_idx = {
			let go = || {
				let jobs = &mut self.jobs;
				if let Some((i, space)) = jobs.iter_mut().enumerate().find(|&(_, ref o)| o.is_none()) {
					*space = Some(job);
					return i
				}
				let len = jobs.len();
				jobs.push(Some(job));
				len
			};
			go()
		};
		JobDescriptor { job_idx: job_idx, job_set: self }
	}

	pub fn new() -> JobSet {
		JobSet { jobs: vec![], events: vec![] }
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
			let jobs = &mut self.job_set.jobs;
			jobs[self.job_idx] = None;
			let len = jobs.iter().enumerate().rev().find(|&(_, pr)| pr.is_some()).map_or(0, |(i, _)| i + 1);
			jobs.truncate(len);
		}
	}
}

impl<'a> JobDescriptor<'a> {
	pub fn job(&self) -> &Job {
		self.job_set.jobs[self.job_idx].as_ref().unwrap()
	}

	pub fn wait(&mut self) {
		let wait_state = self.job().state();
		loop {
			let status = nix::sys::wait::wait().expect("wait errored");
			let job_idx = self.job_set.update_job_set(status);
			let job = self.job_set.jobs[job_idx].as_ref().unwrap();
			if job_idx == self.job_idx && job.state() != wait_state {
				if let (State::Terminated, WaitStatus::Exited(..)) = (job.state(), job.proccesses.last().unwrap().status) {
					self.job_set.events.pop();
				}
				return;
			}
		}
	}
}
