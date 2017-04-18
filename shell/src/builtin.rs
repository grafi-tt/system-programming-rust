use global;

pub fn builtin_cd(_: &mut global::State, _: &Vec<&[u8]>) -> u8 {
	0
}

pub fn builtin_rehash(state: &mut global::State, _: &Vec<&[u8]>) -> u8 {
	state.search_cache.rehash();
	0
}

pub fn match_builtin(name: &[u8]) -> Option<fn(&mut global::State, &Vec<&[u8]>) -> u8> {
	match name {
		b"cd" => Some(builtin_cd),
		b"rehash" => Some(builtin_rehash),
		_ => None,
	}
}
