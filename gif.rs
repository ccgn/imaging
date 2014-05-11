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
	local_table: Option<~[u8]>,

	pub image: ~[u8],

	backgroud_index: u8,
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

			backgroud_index: 0,
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

		println!("image top {0} left {1}", image_top, image_left);
		println!("image width {0} height {1}", image_width, image_height);

		if interlace {
			fail!("interlace not implemented")
		}

		if local_table {
			println!("local table exists");
			let n = 1 << (table_size + 1);
			let b = try!(self.r.read_exact(3 * n));
			self.local_table = Some(b);
		}

		let b = try!(self.read_image_data());
		expand_image(self.global_table,
			     b,
			     image_top as uint,
			     image_left as uint,
			     image_width as uint,
			     image_height as uint,
			     self.width as uint * 3,
			     self.image);

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
		println!("\nGRAPHIC CONTROL EXTENSION");

		let size   = try!(self.r.read_u8());
		assert!(size == 4);

		let fields = try!(self.r.read_u8());
		let delay  = try!(self.r.read_le_u16());
		let trans  = try!(self.r.read_u8());

		println!("size {}", size);
		println!("fields {:0>8t}", fields);
		println!("delay {}ms", delay * 10);
		println!("transparent index {0} - {1}", trans, fields & 1 != 0);

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
		println!("w {0} h {1}", self.width, self.height);

		let fields = try!(self.r.read_u8());

		let global_table = fields & 0x80 != 0;
		let entries = if global_table { 1 << ((fields & 7) + 1)}
			      else {0};

		if global_table {
			self.backgroud_index = try!(self.r.read_u8());
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
		image: &mut [u8]) {

	println!("indices len {0}, 3x {1}", indices.len(), 3 * indices.len());
	println!("image len {}", image.len());

	for y in range(0, height) {
		for x in range(0, width) {
			let index = indices[y * width + x];
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