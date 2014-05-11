extern crate collections;
extern crate time;
extern crate flate;

use std::os;
use std::io::File;
use std::io::MemReader;

use jpeg::JPEGDecoder;
use jpeg::JPEGEncoder;
use png::PNGDecoder;
use png::PNGEncoder;
use gif::GIFDecoder;
use ppm::PPMEncoder;

mod colortype;
mod hash;
mod deflate;
mod zlib;
mod lzw;
mod dct;
mod jpeg;
mod png;
mod gif;
mod ppm;

fn main() {
	let file = if os::args().len() == 2 {
		os::args()[1]
	} else {
		fail!("provide a file")
	};

	let mut fin = File::open(&Path::new(file.clone())).unwrap();
	let buf = fin.read_to_end().unwrap();

	let m = MemReader::new(buf);

	let now = time::precise_time_ns();
	let (out, w, h, c) = match file.split('.').last() {
		Some("jpg") | Some("jpeg") => {
			let mut j = JPEGDecoder::new(m);

			let a = j.decode_image().unwrap();
			let (b, c) = j.dimensions();
			let d = j.color_type();

			(a, b, c, d)
		}
		Some("png") => {
			let mut p = PNGDecoder::new(m);

			let a = p.decode_image().unwrap();
			let (b, c) = p.dimensions();
			let d = p.color_type();
			let _ = p.palette();

			(a, b, c, d)
		}
		Some("gif") => {
			let mut g = GIFDecoder::new(m);

			//Decode first image only
			//Call again to decode successive images
			let a = g.decode_image().unwrap();
			let (b, c) = g.dimensions();
			let d = g.color_type();
			let _ = g.delay();

			(a, b, c, d)
		}
		_ => fail!("unimplemented image extension")
	};
	let after = time::precise_time_ns();

	println!("{0} x {1} pixels", w, h);
	println!("{:?}", c);
	println!("{} bytes", out.len());
	println!("decoded in {} ms", (after - now) / (1000 * 1000));

	let t = out.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(os::args()[1] + ".jpg")).unwrap();

		let now = time::precise_time_ns();
		let _ = JPEGEncoder::new(fout).encode(t.as_slice(), w, h, c);
		let after = time::precise_time_ns();

		println!("encoded jpeg in {} ms", (after - now) / (1000 * 1000));
	});

	let t = out.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(os::args()[1] + ".ppm")).unwrap();

		let now = time::precise_time_ns();
		let _ = PPMEncoder::new(fout).encode(t.as_slice(), w, h, c);
		let after = time::precise_time_ns();

		println!("encoded ppm in {} ms", (after - now) / (1000 * 1000));
	});

	let t = out.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(os::args()[1] + ".png")).unwrap();

		let now = time::precise_time_ns();
		let _ = PNGEncoder::new(fout).encode(t.as_slice(), w, h, c);
		let after = time::precise_time_ns();

		println!("encoded png in {} ms", (after - now) / (1000 * 1000));
	});
}