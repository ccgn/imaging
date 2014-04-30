use std::io;
use std::io::IoResult;
use std::io::MemReader;
use std::cmp;
use std::str;
use std::slice;

use colortype;
use hash::Crc32;
use zlib::ZlibDecoder;

static PNGSIGNATURE: &'static [u8] = &[137, 80, 78, 71, 13, 10, 26, 10];

#[deriving(Eq)]
enum PNGState {
	Start,
	HaveSignature,
	HaveIHDR,
	HavePLTE,
	HaveFirstIDat,
	HaveLastIDat,
	HaveIEND
}

enum PNGError {
	UnknownCompressionMethod,
	UnknownFilterMethod,
	InvalidDimensions,
	InvalidPixelValue,
	InvalidPLTE
}

pub struct PNGDecoder<R> {
	pub palette: Option<~[(u8, u8, u8)]>,

	z: ZlibDecoder<IDATReader<R>>,	
	crc: Crc32,
	previous: ~[u8],
	state: PNGState,

	width: u32,
	height: u32,
	bit_depth: u8,
	colour_type: u8,
	pixel_type: colortype::ColorType,

	compression_method: u8,
	filter_method: u8,
	interlace_method: u8,
	
	chunk_length: u32,
	chunk_type: ~[u8],
	bpp: uint,
}

impl<R: Reader> PNGDecoder<R> {
	pub fn new(r: R) -> PNGDecoder<R> {
		let idat_reader = IDATReader::new(r);
		PNGDecoder {
			pixel_type: colortype::Grey(1),
			palette: None,

			previous: ~[],
			state: Start,
			z: ZlibDecoder::new(idat_reader),
			crc: Crc32::new(),

			width: 0,
			height: 0,
			bit_depth: 0,
			colour_type: 0,
			compression_method: 0,
			filter_method: 0,
			interlace_method: 0,

			chunk_length: 0,
			chunk_type: ~[],
			bpp: 0
		}
	}

	pub fn dimensions(&self) -> (u32, u32) {
		(self.width, self.height)
	}

	pub fn color_type(&self) -> colortype::ColorType {
		self.pixel_type
	}

	pub fn read_scanline(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		if self.state == Start {
			let _ = try!(self.read_metadata());
		}

		let filter  = try!(self.z.read_byte());
		let mut have = 0;	
		
		while have < buf.len() {
			let r = try!(self.z.read(buf.mut_slice_from(have)));
			have += r;
		}
		assert!(have == buf.len());
		unfilter_scanline(filter, self.bpp, self.previous, buf);
		
		slice::bytes::copy_memory(self.previous, buf);

		Ok(buf.len())
	}

	pub fn decode_image(&mut self) -> IoResult<~[u8]> {
		if self.state == Start {
			let _ = try!(self.read_metadata());
		}

		let mut buf = slice::from_elem(self.width as uint * 
									   self.height as uint * 
									   self.bpp, 0u8);
		
		let r = self.width * self.bpp as u32;
		for chunk in buf.mut_chunks(r as uint) {
			let _len = try!(self.read_scanline(chunk));
		}

		Ok(buf)
	}

	fn read_signature(&mut self) -> IoResult<bool> {
		let png = try!(self.z.inner().r.read_exact(8)); 
		
		Ok(png.as_slice() == PNGSIGNATURE)
	}

	fn parse_IHDR(&mut self, buf: ~[u8]) -> Result<(), PNGError> {
		self.crc.update(buf.as_slice());
		let mut m = MemReader::new(buf);

		self.width = m.read_be_u32().unwrap();
		self.height = m.read_be_u32().unwrap();

		if self.width < 0 || self.height < 0 {
			return Err(InvalidDimensions)
		}
		
		self.bit_depth = m.read_byte().unwrap();
		self.colour_type = m.read_byte().unwrap();

		self.pixel_type = match (self.colour_type, self.bit_depth) {
			(0, 1)  => colortype::Grey(1),
			(0, 2)  => colortype::Grey(2),
			(0, 4)  => colortype::Grey(4),
			(0, 8)  => colortype::Grey(8),
			(0, 16) => colortype::Grey(16),
			(2, 8)  => colortype::RGB(8),
			(2, 16) => colortype::RGB(16),
			(3, 1)  => colortype::Palette(1),
			(3, 2)  => colortype::Palette(2),
			(3, 4)  => colortype::Palette(4),
			(3, 8)  => colortype::Palette(8),
			(4, 8)  => colortype::GreyA(8),
			(4, 16) => colortype::GreyA(16),
			(6, 8)  => colortype::RGBA(8),
			(6, 16) => colortype::RGBA(16),
			(_, _)  => return Err(InvalidPixelValue)
		};

		self.compression_method = m.read_byte().unwrap();
		if self.compression_method != 0 {
			return Err(UnknownCompressionMethod)
		}

		self.filter_method = m.read_byte().unwrap();
		if self.filter_method != 0 {
			return Err(UnknownFilterMethod)
		}

		self.interlace_method = m.read_byte().unwrap();
		if self.interlace_method != 0 {
			fail!("Interlace not implemented")
		}

		let channels = match self.colour_type {
			0 => 1,
			2 => 3,
			3 => 3,
			4 => 2,
			6 => 4,
			_ => fail!("unknown colour type")
		};

		self.bpp = ((channels * self.bit_depth + 7) / 8) as uint; 
		self.previous = slice::from_elem(self.bpp * self.width as uint, 0u8);
		Ok(())
	}

	fn parse_PLTE(&mut self, buf: ~[u8]) -> Result<(), PNGError> {
		self.crc.update(buf.as_slice());

		let len = buf.len() / 3;	
		
		if len > 256 || len > (1 << self.bit_depth) || buf.len() % 3 != 0{
			return Err(InvalidPLTE)
		} 

		let p = slice::from_fn(256, |i| {
			if i < len {
				let r = buf[3 * i];
				let g = buf[3 * i + 1];
				let b = buf[3 * i + 2];

				(r, g, b)
			}
			else {
				(0, 0, 0)
			}
		});

		self.palette = Some(p);

		Ok(())
	}

	fn read_metadata(&mut self) -> IoResult<()> {
		assert!(self.state == Start);

		if !try!(self.read_signature()) {
			fail!("Wrong signature")
		}

		self.state = HaveSignature;

		loop {
			let length = try!(self.z.inner().r.read_be_u32());
			let chunk = try!(self.z.inner().r.read_exact(4));
			
			self.chunk_length = length;
			self.chunk_type   = chunk.clone();

			self.crc.update(chunk);
			
			let s = {
				let a = str::from_utf8_owned(self.chunk_type.clone());
				if a.is_none() {
					fail!("FIXME")
				}
				a.unwrap()
			};

			match (s.as_slice(), self.state) {
				("IHDR", HaveSignature) => {
					assert!(length == 13);
					let d = try!(self.z.inner().r.read_exact(length as uint));

					let _ = self.parse_IHDR(d);
					self.state = HaveIHDR;
				}

				("PLTE", HaveIHDR) => {
					let d = try!(self.z.inner().r.read_exact(length as uint));

					let _ = self.parse_PLTE(d);
					self.state = HavePLTE;
				}

				("tRNS", HavePLTE) => {
					assert!(self.palette.is_some());
					fail!("trns unimplemented")
				}
				
				("IDAT", HaveIHDR) if self.colour_type != 3 => {
					self.state = HaveFirstIDat;
					self.z.inner().set_inital_length(self.chunk_length);	
					self.z.inner().crc.update(self.chunk_type.as_slice());

					break;
				}

				("IDAT", HavePLTE) if self.colour_type == 3 => {
					self.state = HaveFirstIDat;
					self.z.inner().set_inital_length(self.chunk_length);
					self.z.inner().crc.update(self.chunk_type.as_slice());
			
					break;
				}

				_ => {
					let b = try!(self.z.inner().r.read_exact(length as uint));
					self.crc.update(b);
				}
			}

			let chunk_crc = try!(self.z.inner().r.read_be_u32());
			let crc = self.crc.checksum();

			assert!(crc == chunk_crc);

			self.crc.reset();
		}

		Ok(())
	}
}

fn unfilter_scanline(filter: u8, bpp: uint, previous: &[u8], scanline: &mut [u8]) {
	let len = scanline.len();
	match filter {
		0 => (),
		
		1 => {
			for i in range(bpp, len) {
				scanline[i] += scanline[i - bpp];
			}
		}

		2 => {
			for i in range(0, len) {
				scanline[i] += previous[i];
			}
		}

		3 => {
			for i in range(0, bpp) {
				scanline[i] += previous[i] / 2;
			}
			for i in range(bpp, len) {
				scanline[i] += ((scanline[i - bpp] as i16 + previous[i] as i16) / 2) as u8;
			}
		}

		4 => {
			for i in range(0, bpp) {
				scanline[i] += filter_paeth(0, previous[i], 0);
			}
			for i in range(bpp, len) {
				scanline[i] += filter_paeth(scanline[i - bpp], previous[i], previous[i - bpp]);
			}
		}
		
		n => fail!("unknown filter type: {}\n", n)
	}
}

fn abs(a: i32) -> i32 {
	if a < 0 {
		a * -1
	} else {
		a
	}
}

fn filter_paeth(a: u8, b: u8, c: u8) -> u8 {
	let ia = a as i32;
	let ib = b as i32;
	let ic = c as i32;

	let p = ia + ib - ic;
	
	let pa = abs(p - ia);
	let pb = abs(p - ib);
	let pc = abs(p - ic);

	if pa <= pb && pa <= pc {
		a
	} else if pb <= pc {
		b
	} else {
		c
	}
}

pub struct IDATReader<R> {
	pub r: R,
	pub crc: Crc32,
	
	eof: bool,
	chunk_length: u32,
}

impl<R:Reader> IDATReader<R> {
	pub fn new(r: R) -> IDATReader<R> {
		IDATReader {
			r: r,
			crc: Crc32::new(),
			eof: false,
			chunk_length: 0,
		}
	}

	pub fn set_inital_length(&mut self, len: u32) {
		self.chunk_length = len;
	}
}

impl<R: Reader> Reader for IDATReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		if self.eof {
			return Err(io::standard_error(io::EndOfFile))
		}

		let len = buf.len();
		let mut start = 0;

		while start < len {
			while self.chunk_length == 0 {
				let chunk_crc = try!(self.r.read_be_u32());
				let crc = self.crc.checksum();

				assert!(crc == chunk_crc);
				self.crc.reset();

				self.chunk_length = try!(self.r.read_be_u32());
				
				let v = try!(self.r.read_exact(4));
				self.crc.update(v.as_slice());

				match str::from_utf8(v.as_slice()) {
					Some("IDAT") => (),
					_ 			 => {
						self.eof = true;
						break
					}			
				}
			}
			
			let m = cmp::min(len - start, self.chunk_length as uint);

			let slice = buf.mut_slice(start, start + m);
			let r = try!(self.r.read(slice));

			start += r;

			self.chunk_length -= r as u32;
			self.crc.update(slice.as_slice());
		}

		Ok(start)
	}
}