use std::io;
use std::cmp;
use std::slice;
use std::io::IoResult;

static MAXCODESIZE: u8 = 12;

pub struct LZWReader<R> {
	r: R,

	dict: ~[Option<~[u8]>],
	prev: ~[u8],

	accumulator: u32,
	num_bits: u8,

	initial_size: u8,
	code_size: u8,

	next_code: u16,
	end: u16,
	clear: u16,

	eof: bool,

	out: ~[u8],
	pos: uint
}

impl<R: Reader> LZWReader<R> {
	pub fn new(r: R, size: u8) -> LZWReader<R> {
		let mut dict = slice::from_elem(1 << MAXCODESIZE, None);

		for i in range(0, 1 << size) {
			dict[i] = Some(~[i as u8])
		}

		LZWReader {
			r: r,

			dict: dict,
			prev: ~[],

			accumulator: 0,
			num_bits: 0,

			initial_size: size,
			code_size: size + 1,

			next_code: (1 << size as u16) + 2,
			clear: 1 << size as u16,
			end: (1 << size as u16) + 1,

			out: ~[],
			pos: 0,

			eof: false
		}
	}

	fn read_code(&mut self) -> IoResult<u16> {
		while self.num_bits < self.code_size {
			let byte = try!(self.r.read_u8());

			self.accumulator |= byte as u32 << self.num_bits;
			self.num_bits += 8;
		}

		let mask = (1 << self.code_size) - 1;
		let code = self.accumulator & mask;

		self.accumulator >>= self.code_size;
		self.num_bits -= self.code_size;

		Ok(code as u16)
	}

	fn decode(&mut self) -> IoResult<()> {
		loop {
			let code = try!(self.read_code());

			if code == self.clear {
				self.code_size = self.initial_size + 1;
				self.next_code = (1 << self.initial_size as u16) + 2;
				self.dict = slice::from_elem(1 << MAXCODESIZE, None);

				for i in range(0, 1 << self.initial_size) {
					self.dict[i] = Some(~[i as u8])
				}

				continue
			}

			else if code == self.end {
				self.eof = true;
				break
			}

			if self.dict[code].is_none() {
				self.dict[code] = Some(self.prev.clone() + ~[self.prev[0]]);
			}

			if self.prev.len() > 0 {
				let mut tmp = self.prev.clone();
				tmp.push(self.dict[code].get_ref()[0]);

				self.dict[self.next_code as uint] = Some(tmp);
				self.next_code += 1;

				if self.next_code >= 1 << self.code_size as u16 {
					self.code_size += 1;
				}
			}

			self.prev = self.dict[code].get_ref().to_owned();

			for (k, &s) in self.dict[code].get_ref().iter().enumerate() {
				self.out.push(s);
			}
		}

		Ok(())
	}
}

impl<R: Reader> Reader for LZWReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		if !self.eof {
			let _ = try!(self.decode());
		}

		if self.pos == self.out.len(){
			return Err(io::standard_error(io::EndOfFile))
		}

		let a = cmp::min(buf.len(), self.out.len() - self.pos);
		slice::bytes::copy_memory(buf, self.out.slice(self.pos, self.pos + a));

		self.pos += a;

		Ok(a)
	}
}