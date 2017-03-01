mod parser;
mod search;
mod job;
mod global;
mod builtin;
mod eval;

extern crate libc;
extern crate nix;

use std::io;
use io::prelude::*;

const PROMPT: &'static [u8] = b"ish> ";

fn main() {
	let mut stdout = io::stdout();
	let stdin = io::stdin();
	let mut stdin_locked = stdin.lock();
	loop {
		let _ = stdout.write(PROMPT);
		let _ = stdout.flush();
		let mut line: Vec<u8> = vec![];
		let _ = stdin_locked.read_until(b'\n', &mut line);
		let pipeline = parser::parse(&line);
		println!("{:?}", pipeline)
	}
}
