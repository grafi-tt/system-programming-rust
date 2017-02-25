use std;
use std::error::Error;

use types::*;

type ParseResult<T> = Result<T, String>;

struct Parser<'a> {
	line: &'a [u8],
	i: usize,
}

impl<'a> Parser<'a> {
	fn proceed_while<F>(&mut self, f: F) where F: Fn(u8) -> bool {
		while let Some(c) = self.line.get(self.i) {
			if !f(*c) { break; }
			self.i += 1;
		}
	}

	fn is_whitespace(c: u8) -> bool {
		match c {
			b' ' | b'\t' | b'\n' => true,
			_ => false,
		}
	}

	fn is_letter(c: u8) -> bool {
		match c {
			b'>' | b'<' | b'&' | b'|' => false,
			_ => !Parser::is_whitespace(c),
		}
	}

	fn is_digit(c: u8) -> bool {
		 b'0' <= c && c <= b'9'
	}

	fn skip_whitespaces(&mut self) {
		self.proceed_while(Parser::is_whitespace);
	}

	fn read_word(&mut self) -> &'a [u8] {
		let orig = self.i;
		self.proceed_while(Parser::is_letter);
		&self.line[orig .. self.i]
	}

	fn read_number(&mut self) -> Option<Result<i32, std::num::ParseIntError>> {
		let orig = self.i;
		self.proceed_while(Parser::is_digit);
		if orig == self.i {
			None
		} else {
			unsafe{
				Some(std::str::from_utf8_unchecked(&self.line[orig .. self.i]).parse())
			}
		}
	}

	fn parse_redirect(&mut self) -> ParseResult<Option<Redirect<'a>>> {
		let orig = self.i;
		let num = self.read_number();

		let typ = match self.line.get(self.i) {
			Some(&b'<') => {
				self.i += 1;
				RedirectType::Input
			},
			Some(&b'>') => if self.line.get(self.i+1) == Some(&b'>') {
				self.i += 2;
				RedirectType::Append
			} else {
				self.i += 1;
				RedirectType::Output
			},
			_ => {
				self.i = orig;
				return Ok(None);
			},
		};

		let from = match num {
			None => if typ == RedirectType::Input { 0 } else { 1 },
			Some(Ok(n)) => n,
			Some(Err(e)) => { return Err(e.description().to_string()); },
		};

		self.skip_whitespaces();
		let target = self.read_word();
		if target.is_empty() {
			return Err("empty redirect".to_string());
		}

		Ok(Some(Redirect { target: target, from: from, typ: typ }))
	}

	fn parse_and_append_redirects(&mut self, redirects: &mut Vec<Redirect<'a>>) -> ParseResult<()> {
		loop {
			match self.parse_redirect() {
				Ok(Some(redirect)) => redirects.push(redirect),
				Ok(None) => { break; },
				Err(e) => { return Err(e); },
			}
			self.skip_whitespaces();
		}
		Ok(())
	}

	fn parse_command(&mut self) -> ParseResult<Command<'a>> {
		let mut redirects: Vec<Redirect<'a>> = vec![];
		let mut arguments: Vec<&'a [u8]> = vec![];

		if let Err(e) = self.parse_and_append_redirects(&mut redirects) {
			return Err(e);
		}

		let name = self.read_word();
		if name.is_empty() {
			return Err("empty command".to_string());
		}

		loop {
			self.skip_whitespaces();
			let word = self.read_word();
			if word.is_empty() {
				break;
			} else {
				arguments.push(word);
			}
		}

		if let Err(e) = self.parse_and_append_redirects(&mut redirects) {
			return Err(e);
		}

		Ok(Command { name: name, arguments: arguments, redirects: redirects })
	}

	fn parse_pipeline(&mut self) -> ParseResult<Pipeline<'a>> {
		let mut commands: Vec<Command<'a>> = vec![];
		let mut is_background = false;

		loop {
			self.skip_whitespaces();
			match self.parse_command() {
				Ok(command) => commands.push(command),
				Err(e) => { return Err(e); },
			}
			match self.line.get(self.i) {
				Some(&b'|') => { self.i += 1; },
				Some(&b'&') => {
					self.i += 1;
					is_background = true;
					self.skip_whitespaces();
					if let Some(&c) = self.line.get(self.i) {
						return Err(format!("character after '&': '{}'", c as char));
					} else {
						break;
					}
				},
				Some(&c) => { return Err(format!("unknown command separator: '{}'", c as char)); },
				None => { break; },
			}
		}
		Ok(Pipeline { commands: commands, is_background: is_background })
	}
}

pub fn parse<'a>(line: &'a [u8]) -> ParseResult<Pipeline<'a>> {
	let mut parser: Parser<'a> = Parser { line: line, i: 0 };
	parser.parse_pipeline()
}
