use std::io::MemWriter;
use std::num;
use std::num::ToStrRadix;

use colortype;
use colortype::{Grey, Palette, GreyA, RGB, RGBA};

pub struct PPMEncoder {
	w: MemWriter
}

impl PPMEncoder {
	pub fn new() -> PPMEncoder {
		PPMEncoder {
			w: MemWriter::new()
		}
	}

	pub fn encode(&mut self, im: &[u8], w: u32, h: u32, c: colortype::ColorType) -> ~[u8] {
		self.write_magic_number();
		self.write_metadata(w, h, c);
		self.write_image(im, c, w, h);

		self.w.get_ref().to_owned()
	}

	fn write_magic_number(&mut self) {
		let _ = self.w.write_str("P6\n");
	}

	fn write_metadata(&mut self, width: u32, height: u32, pixel_type: colortype::ColorType) {
		let w = width.to_str_radix(10);
		let h = height.to_str_radix(10);
		let m = max_pixel_value(pixel_type);

		let _ = self.w.write_str(format!("{0} {1}\n{2}\n", w, h, m));
	}

	fn write_image(&mut self, buf: &[u8], pixel_type: colortype::ColorType, width: u32, height: u32) {
		assert!(buf.len() > 0);
		match pixel_type {
			Grey(8) => {
				for i in range(0, (width * height) as uint) {
					let _ = self.w.write_u8(buf[i]);
					let _ = self.w.write_u8(buf[i]);
					let _ = self.w.write_u8(buf[i]);
				}
			}
			RGB(8) => {
				let _ = self.w.write(buf);
			}
			RGB(16) => {
				let _ = self.w.write(buf);			
			}
			RGBA(8) => {
				for x in buf.chunks(4) {
					let _ = self.w.write_u8(x[0]);
					let _ = self.w.write_u8(x[1]);
					let _ = self.w.write_u8(x[2]);
				}
			}
			a => fail!(format!("not implemented: {:?}", a))
		}
	}
}

fn max_pixel_value(pixel_type: colortype::ColorType) -> u16 {
	let max = match pixel_type {
		Grey(n)    => num::pow(2, n as uint) - 1, 
		RGB(n)     => num::pow(2, n as uint) - 1, 
		Palette(n) => num::pow(2, n as uint) - 1, 
		GreyA(n)   => num::pow(2, n as uint) - 1, 
		RGBA(n)    => num::pow(2, n as uint) - 1
	};

	max as u16	
}
