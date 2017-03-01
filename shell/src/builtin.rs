use global;

pub fn builtin_cd(_: &mut global::State, _: &Vec<&[u8]>) -> u8 {
	0
}

pub fn match_builtin(name: &[u8]) -> Option<fn(&mut global::State, &Vec<&[u8]>) -> u8> {
	match name {
		b"cd" => Some(builtin_cd),
		_ => None,
	}
}
