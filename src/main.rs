extern crate image;

use std::os;
use std::io::File;

use image::image::Image;
use image::GenericImage;
use image::ImageOps;

fn main() {
	let file = if os::args().len() == 2 {
		os::args().as_slice()[1].clone()
	} else {
		fail!("Please enter a file")
	};

	let im = image::open(&Path::new(file.clone())).unwrap();

	println!("dimensions {}", im.dimensions());
	println!("{}", im.color());

	let t = match im {
		image::image::ImageRgb8(a) => a,
		_   	     => fail!("blah!")
	};

	spawn(proc() {
		let mut t = t;
		let fout = File::create(&Path::new(format!("{}.png", os::args().as_slice()[1]))).unwrap();
		t.invert();

		let g = image::image::ImageRgb8(t);
		g.save(fout, image::PNG);
	});
}