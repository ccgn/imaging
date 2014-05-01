use std::io;
use std::io::IoResult;
use std::io::MemReader;
use std::io::MemWriter;
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

static NOFILTER: u8 = 0;
static SUB: u8 = 1;
static UP: u8 = 2;
static AVERAGE: u8 = 3;
static PAETH: u8 = 4;

pub struct PNGDecoder<R> {
	z: ZlibDecoder<IDATReader<R>>,	
	crc: Crc32,
	previous: ~[u8],
	state: PNGState,

	width: u32,
	height: u32,
	bit_depth: u8,
	colour_type: u8,
	pixel_type: colortype::ColorType,
	palette: Option<~[(u8, u8, u8)]>,

	interlace_method: u8,
	
	chunk_length: u32,
	chunk_type: ~[u8],
	bpp: uint,
	rowlength: uint,
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
			interlace_method: 0,

			chunk_length: 0,
			chunk_type: ~[],
			bpp: 0,
			rowlength: 0,
		}
	}

	pub fn dimensions(&self) -> (u32, u32) {
		(self.width, self.height)
	}

	pub fn color_type(&self) -> colortype::ColorType {
		self.pixel_type
	}

	pub fn read_scanline(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		assert!(buf.len() >= self.rowlength);

		if self.state == Start {
			let _ = try!(self.read_metadata());
		}

		let filter  = try!(self.z.read_byte());
		let _ = try!(self.z.fill(buf.mut_slice_to(self.rowlength)));
				
		unfilter(filter, self.bpp, self.previous, buf.mut_slice_to(self.rowlength));
		slice::bytes::copy_memory(self.previous, buf.slice_to(self.rowlength));

		if self.bit_depth < 8 {
			//expand_bit_depth(buf, self.rowlength, self.bit_depth);	
			fail!("unimplemented")
		}

		if self.palette.is_some() {
			let s = (*self.palette.get_ref()).as_slice();
			expand_palette(buf, s, self.rowlength);
		}		

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

	pub fn palette<'a>(&'a self) -> &'a [(u8, u8, u8)] {
		match self.palette {
			Some(ref p) => p.as_slice(),
			None        => &[]
		}
	} 

	pub fn rowlength(&self) -> uint {
		self.bpp * self.width as uint
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
			(3, 1)  => colortype::RGB(8),
			(3, 2)  => colortype::RGB(8),
			(3, 4)  => colortype::RGB(8),
			(3, 8)  => colortype::RGB(8),
			(4, 8)  => colortype::GreyA(8),
			(4, 16) => colortype::GreyA(16),
			(6, 8)  => colortype::RGBA(8),
			(6, 16) => colortype::RGBA(16),
			(_, _)  => return Err(InvalidPixelValue)
		};

		let compression_method = m.read_byte().unwrap();
		if compression_method != 0 {
			return Err(UnknownCompressionMethod)
		}

		let filter_method = m.read_byte().unwrap();
		if filter_method != 0 {
			return Err(UnknownFilterMethod)
		}

		self.interlace_method = m.read_byte().unwrap();
		if self.interlace_method != 0 {
			fail!("Interlace not implemented")
		}

		let channels = match self.colour_type {
			0 => 1,
			2 => 3,
			3 => 1,
			4 => 2,
			6 => 4,
			_ => fail!("unknown colour type")
		};

		self.bpp = ((channels * self.bit_depth + 7) / 8) as uint;
		self.rowlength = self.width as uint * self.bpp; 
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
					
					self.bpp = 3;
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

fn expand_palette(buf: &mut[u8], palette: &[(u8, u8, u8)], entries: uint) {
	assert!(buf.len() == entries * 3);
	let tmp = slice::from_fn(entries, |i| buf[i]);

	for (chunk, &i) in buf.mut_chunks(3).zip(tmp.iter()) {
		let (r, g, b) = palette[i];
		chunk[0] = r;
		chunk[1] = g;
		chunk[2] = b;
	}
}

fn unfilter(filter: u8, bpp: uint, previous: &[u8], current: &mut [u8]) {
	assert!(previous.len() == current.len());
	let len = current.len();
	
	match filter {
		NOFILTER => (),
		
		SUB => {
			for i in range(bpp, len) {
				current[i] += current[i - bpp];
			}
		}

		UP => {
			for i in range(0, len) {
				current[i] += previous[i];
			}
		}

		AVERAGE => {
			for i in range(0, bpp) {
				current[i] += previous[i] / 2;
			}
			for i in range(bpp, len) {
				current[i] += ((current[i - bpp] as i16 + previous[i] as i16) / 2) as u8;
			}
		}

		PAETH => {
			for i in range(0, bpp) {
				current[i] += filter_paeth(0, previous[i], 0);
			}
			for i in range(bpp, len) {
				current[i] += filter_paeth(current[i - bpp], previous[i], previous[i - bpp]);
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

pub struct PNGEncoder<W> {
	w: W,
	crc: Crc32
}

impl<W: Writer> PNGEncoder<W> {
	pub fn new(w: W) -> PNGEncoder<W> {
		PNGEncoder {
			w: w,
			crc: Crc32::new()
		}
	}

	pub fn encode(&mut self, 
				  image: &[u8], 
				  width: u32, 
				  height: u32, 
				  c: colortype::ColorType) -> IoResult<()> {
		
		let _ = try!(self.write_signature());

		let (bytes, bpp) = build_IHDR(width, height, c);
		let _ = try!(self.write_chunk("IHDR", bytes)); 
		
		let compressed_bytes = build_IDAT(image, bpp, width, height);
		
		for chunk in compressed_bytes.chunks(1024 * 256) {
			let _ = try!(self.write_chunk("IDAT", chunk));
		}

		self.write_chunk("IEND", [])
	}

	fn write_signature(&mut self) -> IoResult<()> {
		self.w.write(PNGSIGNATURE)
	}

	fn write_chunk(&mut self, name: &str, buf: &[u8]) -> IoResult<()> {
		self.crc.reset();
		self.crc.update(name);
		self.crc.update(buf.as_slice());

		let crc = self.crc.checksum();

		let _ = try!(self.w.write_be_u32(buf.len() as u32));
		let _ = try!(self.w.write_str(name));

		let _ = try!(self.w.write(buf));
		let _ = try!(self.w.write_be_u32(crc));

		Ok(())
	}
}

fn build_IHDR(width: u32, height: u32, c: colortype::ColorType) -> (~[u8], uint) {
	let mut m = MemWriter::with_capacity(13);
	
	let _ = m.write_be_u32(width);
	let _ = m.write_be_u32(height);

	let (colortype, bit_depth) = match c {
		colortype::Grey(1)    => (0, 1),
		colortype::Grey(2)    => (0, 2),
		colortype::Grey(4)    => (0, 4),
		colortype::Grey(8)    => (0, 8),
		colortype::Grey(16)   => (0, 16),
		colortype::RGB(8)     => (2, 8),
		colortype::RGB(16)    => (2, 16),
		colortype::Palette(1) => (3, 1),
		colortype::Palette(2) => (3, 2),
		colortype::Palette(4) => (3, 4),
		colortype::Palette(8) => (3, 8),
		colortype::GreyA(8)   => (4, 8),
		colortype::GreyA(16)  => (4, 16),
		colortype::RGBA(8)    => (6, 8),
		colortype::RGBA(16)   => (6, 16),
		_ => fail!("unsupported color type and bitdepth")
	};

	let _ = m.write_u8(bit_depth);
	let _ = m.write_u8(colortype);
	
	//compression method, filter method and interlace
	let _ = m.write_u8(0);
	let _ = m.write_u8(0);
	let _ = m.write_u8(0);

	let channels = match colortype {
		0 => 1,
		2 => 3,
		3 => 3,
		4 => 2,
		6 => 4,
		_ => fail!("unknown colour type")
	};

	let bpp = ((channels * bit_depth + 7) / 8) as uint; 

	(m.unwrap(), bpp)
}

fn filter(method: u8, bpp: uint, previous: &[u8], current: &mut [u8]) {
	let len  = current.len();
	let orig = slice::from_fn(len, |i| current[i]);
	
	match method {
		NOFILTER => (),
		SUB      => {
			for i in range(bpp, len) {
				current[i] = orig[i] - orig[i - bpp];
			}
		}
		UP       => {
			for i in range(0, len) {
				current[i] = orig[i] - previous[i];
			}
		}
		AVERAGE  => {
			for i in range(0, bpp) {
				current[i] = orig[i] - previous[i] / 2;
			}

			for i in range(bpp, len) {
				current[i] = orig[i] - ((orig[i - bpp] as i16 + previous[i] as i16) / 2) as u8;
			}
		}
		PAETH    => {
			for i in range(0, bpp) {
				current[i] = orig[i] - filter_paeth(0, previous[i], 0);
			}
			for i in range(bpp, len) {
				current[i] = orig[i] - filter_paeth(orig[i - bpp], previous[i], previous[i - bpp]);
			}
		}

		n => fail!(format!("unknown filter {}", n))
	}
}

fn sum_abs_difference(buf: &[u8]) -> i32 {
	buf.iter().fold(0i32, |sum, &b| sum + if b < 128 {b as i32} else {256 - b as i32})
}

fn select_filter(rowlength: uint, bpp: uint, previous: &[u8], current_s: &mut [u8]) -> u8 {
	let mut sum    = sum_abs_difference(current_s.slice_to(rowlength));
	let mut method = NOFILTER;

	for (i, current) in current_s.mut_chunks(rowlength).enumerate() {
		filter(i as u8 + 1, bpp, previous, current);

		let this_sum = sum_abs_difference(current);
		if this_sum < sum {
			sum = this_sum;
			method = i as u8 + 1;
		} 
	}

	method
}

fn build_IDAT(image: &[u8], bpp: uint, width: u32, height: u32) -> ~[u8] {
	use flate::deflate_bytes_zlib;

	let rowlen = bpp * width as uint;

	let mut p = slice::from_elem(rowlen, 0u8);
	let mut c = slice::from_elem(4 * rowlen, 0u8);
	let mut b = slice::from_elem(height as uint + rowlen * height as uint, 0u8);

	for (row, outrow) in image.chunks(rowlen).zip(b.mut_chunks(1 + rowlen)) {
		for s in c.mut_chunks(rowlen) {
			slice::bytes::copy_memory(s, row);
		}

		let filter = select_filter(rowlen, bpp, p, c);
		
		outrow[0]  = filter;
		let out    = outrow.mut_slice_from(1);
		let stride = (filter as uint - 1) * rowlen;

		match filter {
			NOFILTER => slice::bytes::copy_memory(out, row),
			_ 	 	 => slice::bytes::copy_memory(out, c.slice(stride, stride + rowlen)),
		}

		slice::bytes::copy_memory(p, row);
	}

	deflate_bytes_zlib(b).as_slice().to_owned()
}