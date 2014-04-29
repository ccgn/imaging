use std::io;
use std::io::IoResult;

use deflate::Inflater;

enum ZlibState {Start, CompressedData, End}

pub struct ZlibDecoder<R> {
	inflate: Inflater<R>,
	state: ZlibState,
	s1: u32,
	s2: u32
}

impl<R: Reader> ZlibDecoder<R> {
	pub fn new(r: R) -> ZlibDecoder<R> {
		ZlibDecoder {
			inflate: Inflater::new(r),
			state: Start,
			s1: 1,
			s2: 0,
		}
	}

	pub fn inner<'a>(&'a mut self) -> &'a mut R {
		self.inflate.inner()
	} 

	fn read_header(&mut self) -> IoResult<()> {
		let cmf = try!(self.inner().read_u8());
		let _cm = cmf & 0x0F;
		let _cinfo = cmf >> 4;

		let flg = try!(self.inner().read_u8());
		let fdict  = (flg & 0b10000) == 0;
		if fdict {
			let _dictid = try!(self.inner().read_be_u32());
			fail!("unimplemented")
		}

		assert!((cmf as u16 * 256 + flg as u16) % 31 == 0);

		Ok(()) 
	}

	fn read_checksum(&mut self) -> IoResult<()> {
		let adler32 = try!(self.inner().read_be_u32());
		let sum = (self.s2 << 16) | self.s1;

		assert!(adler32 == sum);

		Ok(())
	}
}

impl<R: Reader> Reader for ZlibDecoder<R> {
	fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		match self.state {
			CompressedData => {
				match self.inflate.read(buf) {
					Ok(n) => {
						for &i in buf.slice_to(n).iter() {
							self.s1 = self.s1 + i as u32;
							self.s2 = self.s1 + self.s2;

							self.s1 %= 65521;
							self.s2 %= 65521;
						}

						if self.inflate.eof() {
							let _ = try!(self.read_checksum()); 
							self.state = End;
						}

						Ok(n)
					}

					e => e
				}
			}

			Start => {
				let _ = try!(self.read_header());
				self.state = CompressedData;
				self.read(buf)
			}

			End => Err(io::standard_error(io::EndOfFile))
		}
	}
}