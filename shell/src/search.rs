use std::collections::HashMap;
use std::ffi::{CStr,CString};

pub struct SearchCache {
	imp: HashMap<CString, CString>
}

impl SearchCache {
	pub fn rehash() -> SearchCache {
		// TODO
		let mut h = HashMap::new();
		h.insert(CString::new("ls").unwrap(), CString::new("/usr/bin/ls").unwrap());
		SearchCache { imp: h }
	}
	pub fn lookup(&self, name: &CString) -> Option<&CString> {
		self.imp.get(name)
	}
}
