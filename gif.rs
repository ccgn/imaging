extern crate flate;

use std::os;
use std::slice;
use std::io::File;
use std::io::MemReader;
use std::io::IoResult;

use lzw::LZWReader;

use png::PNGEncoder;

mod lzw;
mod colortype;
mod deflate;
mod zlib;
mod hash;
mod png;

static IMAGEDESCRIPTOR: u8 = 0x2C;
static EXTENSION: u8 = 0x21;
static APPLICATION: u8 = 0xFF;
static GRAPHICCONTROL: u8 = 0xF9;
static COMMENT: u8 = 0xFE;
static TRAILER: u8 = 0x3B;

struct GIFDecoder <R> {
	r: R,

	pub width: u16,
	pub height: u16,

	global_table: [(u8, u8, u8), ..256],
	local_table: Option<~[(u8, u8, u8)]>,

	pub image: ~[u8],

	global_backgroud_index: Option<u8>,
	local_transparent_index: Option<u8>,
}

impl<R: Reader> GIFDecoder<R> {
	pub fn new(r: R) -> GIFDecoder<R> {
		GIFDecoder {
			r: r,

			width: 0,
			height: 0,

			global_table: [(0u8, 0u8, 0u8), ..256],
			local_table: None,

			image: ~[],

			global_backgroud_index: None,
			local_transparent_index: None
		}
	}

	fn read_header(&mut self) -> IoResult<()> {
		let signature = try!(self.r.read_exact(3));
		let version   = try!(self.r.read_exact(3));

		assert!(signature.as_slice() == "GIF".as_bytes());
		assert!(version.as_slice() == "87a".as_bytes() ||
			version.as_slice() == "89a".as_bytes());

		Ok(())
	}

	fn read_block(&mut self) -> IoResult<~[u8]> {
		let size = try!(self.r.read_u8());
		self.r.read_exact(size as uint)
	}

	fn read_image_data(&mut self) -> IoResult<~[u8]> {
		let minimum_code_size = try!(self.r.read_u8());
		assert!(minimum_code_size <= 8);
		let mut data = ~[];

		loop {
			let b = try!(self.read_block());
			if b.len() == 0 {
				break
			}

			data = data + b;
		}

		let m = MemReader::new(data);
		let mut lzw = LZWReader::new(m, minimum_code_size);
		let b = lzw.read_to_end().unwrap();

		Ok(b)
    	}

	fn read_image_descriptor(&mut self) -> IoResult<()> {
		let image_left   = try!(self.r.read_le_u16());
		let image_top    = try!(self.r.read_le_u16());
		let image_width  = try!(self.r.read_le_u16());
		let image_height = try!(self.r.read_le_u16());

		let fields = try!(self.r.read_u8());

		let local_table = fields & 80 != 0;
		let interlace   = fields & 40 != 0;
		let table_size  = fields & 7;

		if interlace {
			fail!("interlace not implemented")
		}

		if local_table {
			let n   = 1 << (table_size + 1);
			let buf = try!(self.r.read_exact(3 * n));
			let mut b = slice::from_elem(n, (0u8, 0u8, 0u8));

			for (i, rgb) in buf.chunks(3).enumerate() {
				b[i] = (rgb[0], rgb[1], rgb[2]);
			}

			self.local_table = Some(b);
		}

		let indices = try!(self.read_image_data());

		let trans_index = if self.local_transparent_index.is_some() {
			self.local_transparent_index
		} else {
			self.global_backgroud_index
		};

		let table = if self.local_table.is_some() {
			self.local_table.get_ref().as_slice()
		} else {
			self.global_table.as_slice()
		};

		expand_image(table,
			     indices,
			     image_top as uint,
			     image_left as uint,
			     image_width as uint,
			     image_height as uint,
			     self.width as uint * 3,
			     trans_index,
			     self.image);

		self.local_table = None,
		self.local_transparent_index = None

		Ok(())
	}

	fn read_extension(&mut self) -> IoResult<()> {
		let identifier = try!(self.r.read_u8());

		match identifier {
		    APPLICATION    => try!(self.read_application_extension()),
		    GRAPHICCONTROL => try!(self.read_graphic_control_extension()),
		    _ => {
		    	let mut data = ~[];

			loop {
				let b = try!(self.read_block());
				if b.len() == 0 {
					break
				}

				data = data + b;
			}

		    }
		}

		Ok(())
	}

	fn read_graphic_control_extension(&mut self) -> IoResult<()> {
		let size   = try!(self.r.read_u8());
		assert!(size == 4);

		let fields = try!(self.r.read_u8());
		let delay  = try!(self.r.read_le_u16());
		let trans  = try!(self.r.read_u8());

		println!("delay {}ms", delay * 10);

		if fields & 1 != 0 {
			self.local_transparent_index = Some(trans);
		}

		let disposal = (fields & 0x1C) >> 2;
		match disposal {
			0 => println!("no disposal"),
			1 => println!("do not dispose"),
			2 => println!("restore to background"),
			3 => println!("restore to prev"),
			_ => println!("undefined")
		}

		let _term = try!(self.r.read_u8());

		Ok(())
	}

	fn read_application_extension(&mut self) -> IoResult<()> {
		let size = try!(self.r.read_u8());
		let _ = try!(self.r.read_exact(size as uint));

		loop {
			let b = try!(self.read_block());
			if b.len() == 0 {
				break
		    	}
		}

		Ok(())
	}

	fn read_logical_screen_descriptor(&mut self) -> IoResult<()> {
		self.width  = try!(self.r.read_le_u16());
		self.height = try!(self.r.read_le_u16());
		self.image  = slice::from_elem(self.width as uint *
					       self.height as uint *
					       3, 0u8);

		let fields = try!(self.r.read_u8());

		let global_table = fields & 0x80 != 0;
		let entries = if global_table { 1 << ((fields & 7) + 1)}
			      else {0};

		if global_table {
			let b = try!(self.r.read_u8());
			self.global_backgroud_index = Some(b);
		}

		let _aspect_ratio = try!(self.r.read_u8());

		let buf = try!(self.r.read_exact(3 * entries));

		for (i, rgb) in buf.chunks(3).enumerate() {
			self.global_table[i] = (rgb[0], rgb[1], rgb[2]);
		}

		Ok(())
	}

	pub fn decode(&mut self) -> IoResult<()> {
		let _ = try!(self.read_header());
		let _ = try!(self.read_logical_screen_descriptor());
		let mut i = 0;
		loop {
			let byte = try!(self.r.read_u8());

			println!("byte 0x{:X}", byte);

			if byte == EXTENSION {
				let _ = try!(self.read_extension());
			} else if byte == IMAGEDESCRIPTOR {
				let _ = try!(self.read_image_descriptor());

				let fout = File::create(&Path::new(os::args()[1] + format!("-{}", i) + ".png")).unwrap();

				let _ = PNGEncoder::new(fout).encode(self.image.as_slice(),
								     self.width as u32,
								     self.height as u32,
								     colortype::RGB(8));
				i += 1;
			} else {
				break
			}
		}

		Ok(())
	}
}

fn expand_image(palete: &[(u8, u8, u8)],
		indices: &[u8],
		y0: uint,
		x0: uint,
		width: uint,
		height: uint,
		stride: uint,
		trans_index: Option<u8>,
		image: &mut [u8]) {

	for y in range(0, height) {
		for x in range(0, width) {
			let index = indices[y * width + x];
			if trans_index == Some(index as u8) {
				continue
			}

			let (r, g, b) = palete[index];

			image[(y0 + y) * stride + x0 * 3 + x * 3 + 0] = r;
			image[(y0 + y) * stride + x0 * 3 + x * 3 + 1] = g;
			image[(y0 + y) * stride + x0 * 3 + x * 3 + 2] = b;
		}
	}
}

fn main() {
	let file = if os::args().len() == 2 {
		os::args()[1]
	} else {
		fail!("provide a file")
	};

	let fin = File::open(&Path::new(file.clone())).unwrap();

	let mut g = GIFDecoder::new(fin);
	let _ = g.decode();
}