extern crate image;
extern crate time;

use std::os;
use std::io::File;
use std::io::MemReader;

use image::Image;
use image::{PNG, JPEG, GIF, WEBP, PPM};

fn main() {
	let file = if os::args().len() == 2 {
		os::args().as_slice()[1].clone()
	} else {
		fail!("Please enter a file")
	};

	let mut fin = File::open(&Path::new(file.clone())).unwrap();
	let buf = fin.read_to_end().unwrap();
	let m = MemReader::new(buf);

	let imagetype = match file.as_slice().split('.').last() {
		Some("jpg") |
		Some("jpeg") => JPEG,
		Some("png")  => PNG,
		Some("gif")  => GIF,
		Some("webp") => WEBP,
		_ 	     => fail!("unimplemented image extension")
	};

	let now = time::precise_time_ns();
	let im = Image::load(m, imagetype).unwrap();
	let after = time::precise_time_ns();

	println!("dimensions {}", im.dimensions());
	println!("{}", im.colortype());
	println!("{} bytes", im.raw_pixels().len());
	println!("decoded in {} ms", (after - now) / (1000 * 1000));

	let mut im = im;

	let now = time::precise_time_ns();
	im.invert();
	let after = time::precise_time_ns();

	println!("inverted in {} ms", (after - now) / (1000 * 1000));


	let t = im.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(format!("{}.jpg", os::args().as_slice()[1]))).unwrap();


		let now = time::precise_time_ns();
		let _ = t.save(fout, JPEG);
		let after = time::precise_time_ns();

		println!("encoded jpeg in {} ms", (after - now) / (1000 * 1000));
	});

	let t = im.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(format!("{}.ppm", os::args().as_slice()[1]))).unwrap();

		let now = time::precise_time_ns();
		let _ = t.save(fout, PPM);
		let after = time::precise_time_ns();

		println!("encoded ppm in {} ms", (after - now) / (1000 * 1000));
	});

	spawn(proc() {
		let fout = File::create(&Path::new(format!("{}.png", os::args().as_slice()[1]))).unwrap();

		let now = time::precise_time_ns();
		let _ = im.grayscale().save(fout, PNG);
		let after = time::precise_time_ns();

		println!("encoded png in {} ms", (after - now) / (1000 * 1000));
	});
}