use std::{fs,env,io,process};
use io::prelude::*;
use std::str;

struct TmuxHandler {
	stdin: process::ChildStdin,
	stdout: io::BufReader<process::ChildStdout>,
}

fn sleep_little() {
	use std::time::Duration;
	use std::thread::sleep;
	sleep(Duration::from_millis(500));
}

impl TmuxHandler {
	pub fn new() -> io::Result<TmuxHandler> {
		let mut cmd = process::Command::new("tmux");
		let child = cmd.arg("-C")
			.stdin(process::Stdio::piped())
			.stdout(process::Stdio::piped())
			.spawn()?;
		let mut t = TmuxHandler {
			stdin: child.stdin.unwrap(),
			stdout: io::BufReader::new(child.stdout.unwrap()) };
		t.send(b"new-window target/debug/ish");
		sleep_little();
		Ok(t)
	}

	pub fn send(&mut self, msg: &[u8]) {
		writeln!(io::stderr(), "{}", str::from_utf8(msg).unwrap());
		let _ = self.stdin.write(msg);
		let _ = self.stdin.write(b"\n");
	}

	pub fn read(&mut self) -> Vec<u8> {
		let mut buf: Vec<u8> = vec![];
		let mut len: usize = 0;
		let mut len_with_trail: usize = 0;
		loop {
			match self.stdout.read_until(b'\n', &mut buf) {
				Err(_) => { break; },
				Ok(n) => match buf[len_with_trail] {
					b'%' => {
						if len == 0 {
							buf.clear();
						} else {
							buf.truncate(len-1);
							break;
						}
					},
					b'\n' => {
						len_with_trail += n;
					},
					_ => {
						len_with_trail += n;
						len = len_with_trail;
					},
				}
			}
		}
		buf
	}

	pub fn input(&mut self, content: &[u8]) {
		let mut buf: Vec<u8> = b"send-keys ".to_vec();
		for &c in content {
			match c {
				0x21...0x7e => buf.push(c),
				b' ' => buf.extend(b"Space"),
				_ => panic!("unknown char {}", c),
			}
			buf.push(b' ');
		}
		buf.extend(b"Enter");
		self.send(&buf);
	}

	pub fn capture(&mut self) -> Vec<u8> {
		self.send(b"capture-pane -p -S-");
		self.read()
	}
}

#[test]
fn prompt() {
	let mut t = TmuxHandler::new().unwrap();
	assert_eq!(t.capture(), b"ish>");
}
