use global;

use std::{env,process,str};
use nix::unistd;

const HOME_KEY: &'static str = "HOME";

pub fn builtin_cd(_: &mut global::State, args: &Vec<&[u8]>) -> u8 {
	if args.len() > 2 { return 1; }
	let r = match args.get(0) {
		Some(&dir) => unistd::chdir(dir),
		None => match env::var_os(HOME_KEY) {
			Some(ref dir) => unistd::chdir(dir.as_os_str()),
			None => { return 2; },
		}
	};
	if let Err(e) = r {
		use std::error::Error;
		println!("{}", e.description());
		return 3;
	}
	0
}

pub fn builtin_exit(_: &mut global::State, args: &Vec<&[u8]>) -> u8 {
	let s = args.get(0).and_then(|&a| str::from_utf8(a).ok()).and_then(|s| s.parse().ok()).unwrap_or(0);
	process::exit(s);
}

pub fn builtin_rehash(state: &mut global::State, _: &Vec<&[u8]>) -> u8 {
	state.search_cache.rehash();
	0
}

pub fn match_builtin(name: &[u8]) -> Option<fn(&mut global::State, &Vec<&[u8]>) -> u8> {
	match name {
		b"cd" => Some(builtin_cd),
		b"exit" => Some(builtin_exit),
		b"rehash" => Some(builtin_rehash),
		_ => None,
	}
}
