extern crate image;

use std::os;
use std::io::File;

use image::image::Image;
use image::{PNG, JPEG};

fn main() {
	let file = if os::args().len() == 2 {
		os::args().as_slice()[1].clone()
	} else {
		fail!("Please enter a file")
	};

	let im=Image::open(&Path::new(file.clone())).unwrap();

	println!("dimensions {}", im.dimensions());
	println!("{}", im.color());

	let t = im.clone();
	spawn(proc() {
		let fout = File::create(&Path::new(format!("{}.png", os::args().as_slice()[1]))).unwrap();
		let _    = t.save(fout, PNG);
	});
}