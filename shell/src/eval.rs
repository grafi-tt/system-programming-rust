use parser;
use job;
use global;
use builtin;

use std::{env,error,fmt,fs,ffi,io};
use std::ffi::{CString,OsString,OsStr};
use io::Write;
use nix;
use nix::{unistd,fcntl};
use libc;

#[derive(Debug)]
enum ExecError {
	NixError(nix::Error),
	IoError(io::Error),
	NulError(ffi::NulError),
}
impl From<nix::Error> for ExecError {
	fn from(e: nix::Error) -> ExecError {
		ExecError::NixError(e)
	}
}
impl From<io::Error> for ExecError {
	fn from(e: io::Error) -> ExecError {
		ExecError::IoError(e)
	}
}
impl From<ffi::NulError> for ExecError {
	fn from(e: ffi::NulError) -> ExecError {
		ExecError::NulError(e)
	}
}
impl fmt::Display for ExecError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			ExecError::NixError(ref e) => write!(f, "Nix error: {}", e),
			ExecError::IoError(ref e) => write!(f, "IO error: {}", e),
			ExecError::NulError(ref e) => write!(f, "Nul char error: {}", e),
		}
	}
}
impl error::Error for ExecError {
	fn description(&self) -> &str {
		match *self {
			ExecError::NixError(ref e) => e.description(),
			ExecError::IoError(ref e) => e.description(),
			ExecError::NulError(ref e) => e.description(),
		}
	}
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			ExecError::NixError(ref e) => Some(e),
			ExecError::IoError(ref e) => Some(e),
			ExecError::NulError(ref e) => Some(e),
		}
	}
}

fn do_exec_command(state: &mut global::State, command: &parser::Command, skip_match_builtin: bool) -> Result<u8, ExecError> {
	use std::os::unix::ffi::{OsStrExt,OsStringExt};
	use std::os::unix::io::IntoRawFd;

	for redirect in &command.redirects {
		let mut oopt = fs::OpenOptions::new();
		let _ = match redirect.typ {
			parser::RedirectType::Input => oopt.read(true),
			parser::RedirectType::Output => oopt.write(true).create(true),
			parser::RedirectType::Append => oopt.append(true).create(true),
		};
		let file = oopt.open(OsStr::from_bytes(redirect.target))?;
		let fd = file.into_raw_fd();
		unistd::dup2(fd, redirect.from)?;
		unistd::close(fd)?;
	}
	if !skip_match_builtin {
		if let Some(builtin) = builtin::match_builtin(command.name) {
			return Ok(builtin(state, &command.arguments));
		}
	}
	let cmd_name = CString::new(command.name.to_owned())?;
	let external = match state.search_cache.lookup(&cmd_name) {
		Some(e) => e,
		None => {
			let mut stderr = io::stderr();
			let _ = stderr.write(b"command not found: ");
			let _ = stderr.write(command.name);
			let _ = stderr.write(b"\n");
			let _ = stderr.flush();
			return Ok(127);
		}
	};
	let arguments: Result<Vec<CString>, ffi::NulError> = command.arguments.iter().map(|&s| CString::new(s)).collect();
	let arguments: Vec<CString> = arguments?;
	let environ: Result<Vec<CString>, ffi::NulError> = env::vars_os().map(|(mut k, v)| CString::new({ k.push(OsString::from("=")); k.push(v); k.into_vec() })).collect();
	let environ: Vec<CString> = environ?;
	unistd::execve(external, &arguments, &environ)?;
	unreachable!()
}

fn exec_command(state: &mut global::State, command: &parser::Command, skip_match_builtin: bool) -> ! {
	use std::error::Error;

	let r = do_exec_command(state, command, skip_match_builtin);
	let s = r.unwrap_or_else(|e| {
		let _ = writeln!(&mut io::stderr(), "{}", e.description());
		126
	});
	unsafe{ libc::_exit(s as libc::c_int) }
}

fn do_eval(state: &mut global::State, pipeline: &parser::Pipeline) -> nix::Result<u8> {
	let commands = &pipeline.commands;
	assert!(commands.len() > 0);

	let mut skip_match_builtin = false;
	if commands.len() == 1 && commands[0].redirects.is_empty() {
		if let Some(func) = builtin::match_builtin(commands[0].name) {
			return Ok(func(state, &commands[0].arguments));
		}
		skip_match_builtin = true;
	}

	let mut job_builder = job::JobBuilder::new(commands.len());
	{
		let mut pipe_stdin = 0;
		let mut pipe_stdout = 0;
		let mut pipe_stdout_next = 0;
		let mut is_last = true;
		for i in (0 .. commands.len()).rev() {
			let is_first = i == 0;
			if !is_first {
				let (pipe_read, pipe_write) = unistd::pipe2(fcntl::O_CLOEXEC)?;
				pipe_stdin = pipe_read;
				pipe_stdout = pipe_stdout_next;
				pipe_stdout_next = pipe_write;
			}
			match job_builder.push_fork()? {
				unistd::ForkResult::Parent{..} => {
					if !is_last {
						unistd::close(pipe_stdout)?;
					}
					if !is_first {
						unistd::close(pipe_stdin)?;
					}
				},
				unistd::ForkResult::Child => {
					if !is_last {
						unistd::dup2(pipe_stdout, libc::STDOUT_FILENO)?;
					}
					if !is_first {
						unistd::dup2(pipe_stdin, libc::STDIN_FILENO)?;
					}
					exec_command(state, &commands[i], skip_match_builtin);
				},
			}
			is_last = false;
		}
	}

	use job::WaitStatusExt;
	let mut job_desc = state.job_set.push(job_builder.build());
	job_desc.wait();
	Ok(job_desc.job().proccesses.last().unwrap().status.code())
}

pub fn eval(state: &mut global::State, pipeline: &parser::Pipeline) -> u8 {
	do_eval(state, pipeline).unwrap_or(126)
}
