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
	let mut state = global::State::new();
	loop {
		let _ = stdout.write(PROMPT);
		let _ = stdout.flush();
		let mut line: Vec<u8> = vec![];
		match stdin_locked.read_until(b'\n', &mut line) {
			Ok(0) => { return; },
			Err(e) => {
				use std::error::Error;
				println!("read error: {:?}", e.description());
			},
			Ok(_) => {
				let pipeline = match parser::parse(&line) {
					Ok(p) => p,
					Err(e) => {
						println!("parse error: {:?}", e);
						continue;
					},
				};
				eval::eval(&mut state, &pipeline);
			}
		}
	}
}
