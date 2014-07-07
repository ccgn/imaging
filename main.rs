extern crate image;

use std::os;
use std::io::File;

use image::image::Image;
use image::GenericImage;
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

	let t = match im {
		image::image::ImageRgb8(a) => a,
		_   	     => fail!("blah!")
	};

	spawn(proc() {
		let mut t = t;
		let fout = File::create(&Path::new(format!("{}.png", os::args().as_slice()[1]))).unwrap();
		let sub  = t.crop(0, 0, 400, 400).to_image();

		let g = image::image::ImageRgb8(sub);
		g.save(fout, PNG);
	});
}