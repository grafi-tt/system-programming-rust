#[derive(Debug, PartialEq, Eq)]
pub enum RedirectType { Input, Output, Append }

#[derive(Debug)]
pub struct Redirect<'a> {
	pub target: &'a [u8],
	pub from: i32,
	pub typ: RedirectType,
}

#[derive(Debug)]
pub struct Command<'a> {
	pub name: &'a [u8],
	pub arguments: Vec<&'a [u8]>,
	pub redirects: Vec<Redirect<'a>>,
}

#[derive(Debug)]
pub struct Pipeline<'a> {
	pub commands: Vec<Command<'a>>,
	pub is_background: bool,
}
