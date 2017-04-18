use std::collections::HashMap;
use std::ffi::CString;
use std::{env,fs,io};

pub struct SearchCache {
	imp: HashMap<CString, CString>
}

const PATH_KEY: &'static str = "PATH";

impl SearchCache {
	pub fn new() -> SearchCache {
		let mut this = SearchCache { imp: HashMap::new() };
		this.rehash();
		this
	}
	fn add_entry(&mut self, entry: io::Result<fs::DirEntry>) -> io::Result<()> {
		use std::os::unix::ffi::OsStringExt;
		let e = entry?;
		let file_name = CString::new(e.file_name().into_vec())?;
		let path = CString::new(e.path().into_os_string().into_vec())?;
		self.imp.entry(file_name).or_insert(path);
		Ok(())
	}
	pub fn rehash(&mut self) {
		self.imp.clear();
		for path in env::split_paths(PATH_KEY) {
			if let Ok(entries) = fs::read_dir(path) {
				for entry in entries {
					let _ = self.add_entry(entry);
				}
			}
		}
	}
	pub fn lookup(&self, name: &CString) -> Option<&CString> {
		self.imp.get(name)
	}
}
