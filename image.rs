use std::{
	io,
    cmp,
	slice
};

use std::ascii::StrAsciiExt;
use std::default::Default;

use imaging::{
	sample,
	colorops,
	affine,
	colortype,
	pixel,
};

use imaging::pixel::Pixel;
use imaging::pixelbuf::PixelBuf;
use imaging::colortype::ColorType;

use imaging::pixelbuf::{
	Luma8,
	LumaA8,
	Rgb8,
	Rgba8
};

use codecs::{
	png,
	jpeg,
	webp,
	gif,
	ppm,
};

/// An enumeration of Image Errors
#[deriving(Show, PartialEq, Eq)]
pub enum ImageError {
        ///The Image is not formatted properly
	FormatError,

        ///The Image's dimensions are either too small or too large
	DimensionError,

        ///The Decoder does not support this image format
	UnsupportedError,

        ///The Decoder does not support this color type
	UnsupportedColor,

        ///Not enough data was provided to the Decoder
        ///to decode the image
	NotEnoughData,

        ///An I/O Error occurred while decoding the image
	IoError,

        ///The end of the image has been reached
        ImageEnd
}

pub type ImageResult<T> = Result<T, ImageError>;

/// An enumeration of supported image formats.
/// Not all formats support both encoding and decoding.
#[deriving(PartialEq, Eq, Show)]
pub enum ImageFormat {
	/// An Image in PNG Format
	PNG,

	/// An Image in JPEG Format
	JPEG,

	/// An Image in GIF Format
	GIF,

	/// An Image in WEBP Format
	WEBP,

	/// An Image in PPM Format
	PPM
}

/// The trait that all decoders implement
pub trait ImageDecoder {
        ///Return a tuple containing the width and height of the image
        fn dimensions(&mut self) -> ImageResult<(u32, u32)>;

        ///Return the color type of the image e.g RGB(8) (8bit RGB)
        fn colortype(&mut self) -> ImageResult<ColorType>;

        ///Returns the length in bytes of one decoded row of the image
        fn row_len(&mut self) -> ImageResult<uint>;

        ///Read one row from the image into buf
        ///Returns the row index
        fn read_scanline(&mut self, buf: &mut [u8]) -> ImageResult<u32>;

        ///Decode the entire image and return it as a Vector
        fn read_image(&mut self) -> ImageResult<Vec<u8>>;

        ///Decode a specific region of the image, represented by the rectangle
        ///starting from ```x``` and ```y``` and having ```length``` and ```width```
        fn load_rect(&mut self, x: u32, y: u32, length: u32, width: u32) -> ImageResult<Vec<u8>> {
                let (w, h) = try!(self.dimensions());

                if length > h || width > w || x > w || y > h {
                        return Err(DimensionError)
                }

                let c = try!(self.colortype());

                let bpp = colortype::bits_per_pixel(c) / 8;
                let rowlen  = try!(self.row_len());

                let mut buf = Vec::from_elem(length as uint * width as uint * bpp, 0u8);
                let mut tmp = Vec::from_elem(rowlen, 0u8);

                loop {
                        let row = try!(self.read_scanline(tmp.as_mut_slice()));
                        if row - 1 == y {
                                break
                        }
                }

                for i in range(0, length as uint) {
                        {
                                let from = tmp.slice_from(x as uint * bpp)
                                              .slice_to(width as uint * bpp);

                                let to   = buf.mut_slice_from(i * width as uint * bpp)
                                              .mut_slice_to(width as uint * bpp);

                                slice::bytes::copy_memory(to, from);
                        }

                        let _ = try!(self.read_scanline(tmp.as_mut_slice()));
                }

                Ok(buf)
        }
}

pub trait GenericImage<P> {
        fn from_pixel(width: u32, height: u32, pixel: P) -> Self;
        fn dimensions(&self) -> (u32, u32);
        fn bounds(&self) -> (u32, u32, u32, u32);
        fn get_pixel(&self, x: u32, y: u32) -> P;
        fn put_pixel(&mut self, x: u32, y: u32, pixel: P);
}

/// A Generic representation of an image
#[deriving(Clone, Show)]
pub struct ImageBuf<P> {
	pixels:  Vec<P>,
	xoffset: u32,
	yoffset: u32,
	width:   u32,
	height:  u32,
	vstride: u32,
}

impl<T: Primitive, P: Pixel<T>> ImageBuf<P> {
	///Construct a new Generic Image
	pub fn new(pixels: Vec<P>, width: u32, height: u32) -> ImageBuf<P> {
		ImageBuf {
			pixels:  pixels,
			xoffset: 0,
			yoffset: 0,
			width:   width,
			height:  height,
			vstride: width,
		}
	}

        pub fn pixels<'a>(&'a self) -> &'a [P] {
                self.pixels.as_slice()
        }

        /// Return an immutable view into this image
    pub fn crop<'a, I: GenericImage<P>>(&'a self, x: u32, y: u32, width: u32, height: u32) -> SubImage<'a> {
        let x = cmp::min(x, self.width);
        let y = cmp::min(y, self.height);

        let height = cmp::min(height, self.height - y);
        let width  = cmp::min(width, self.width - x);

        SubImage {
            pixels:  &self.pixels,
            color:   self.color,
            xoffset: x,
            yoffset: y,
            width:   width,
            height:  height,
            vstride: self.width,
        }
    }
}

pub type ImageRGB8  = ImageBuf<pixel::Rgb<u8>>;
pub type ImageRGBA8 = ImageBuf<pixel::Rgba<u8>>;
pub type ImageL8    = ImageBuf<pixel::Luma<u8>>;
pub type ImageLA8   = ImageBuf<pixel::LumaA<u8>>;

#[deriving(Clone)]
pub enum Image {
        ImageLuma8(ImageL8),
        ImageLumaA8(ImageLA8),
        ImageRgb8(ImageRGB8),
        ImageRgba8(ImageRGBA8),
}

impl Image {
        /// Open the image located at the path specified.
        /// The image's format is determined from the path's file extension.
        pub fn open(path: &Path) -> ImageResult<Image> {
                let fin = match io::File::open(path) {
                        Ok(f)  => f,
                        Err(_) => return Err(IoError)
                };

                let ext    = path.extension_str()
                                 .map_or("".to_string(), |s| s.to_ascii_lower());

                let format = match ext.as_slice() {
                        "jpg" |
                        "jpeg" => JPEG,
                        "png"  => PNG,
                        "gif"  => GIF,
                        "webp" => WEBP,
                        _      => return Err(UnsupportedError)
                };

                Image::load(fin, format)
        }

        /// Create a new image from ```r```.
        pub fn load<R: Reader>(r: R, format: ImageFormat) -> ImageResult<Image> {
                match format {
                        PNG  => decoder_to_image(png::PNGDecoder::new(r)),
                        GIF  => decoder_to_image(gif::GIFDecoder::new(r)),
                        JPEG => decoder_to_image(jpeg::JPEGDecoder::new(r)),
                        WEBP => decoder_to_image(webp::WebpDecoder::new(r)),
                        _    => Err(UnsupportedError),
                }
        }

        pub fn raw_pixels(&self) -> Vec<u8> {
                image_to_bytes(self)
        }

        pub fn dimensions(&self) -> (u32, u32) {
                match *self {
                        ImageLuma8(ref a) => a.dimensions(),
                        ImageLumaA8(ref a) => a.dimensions(),
                        ImageRgb8(ref a) => a.dimensions(),
                        ImageRgba8(ref a) => a.dimensions(),
                }
        }

        pub fn color(&self) -> ColorType {
                match *self {
                        ImageLuma8(_) => colortype::Grey(8),
                        ImageLumaA8(_) => colortype::GreyA(8),
                        ImageRgb8(_) => colortype::RGB(8),
                        ImageRgba8(_) => colortype::RGBA(8),
                }
        }

        /// Create a new image from a byte slice
        pub fn load_from_memory(buf: &[u8], format: ImageFormat) -> ImageResult<Image> {
                let b = io::BufReader::new(buf);

                Image::load(b, format)
        }

        /// Encode this image and write it to ```w```
        pub fn save<W: Writer>(&self, w: W, format: ImageFormat) -> io::IoResult<ImageResult<()>> {
                let bytes = self.raw_pixels();
                let (width, height) = self.dimensions();
                let color = self.color();

                let r = match format {
                        PNG  => {
                                let mut p = png::PNGEncoder::new(w);
                                try!(p.encode(bytes.as_slice(),
                                              width,
                                              height,
                                              color))
                                Ok(())
                        }

                        PPM  => {
                                let mut p = ppm::PPMEncoder::new(w);
                                try!(p.encode(bytes.as_slice(),
                                              width,
                                              height,
                                              color))
                                Ok(())
                        }

                        JPEG => {
                                let mut j = jpeg::JPEGEncoder::new(w);
                                try!(j.encode(bytes.as_slice(),
                                              width,
                                              height,
                                              color))
                                Ok(())
                        }

                        _    => Err(UnsupportedError),
                };

                Ok(r)
        }

        /// Return a grayscale version of this image.
        /*pub fn grayscale(&self) -> Image {
                match *self {
                        ImageLuma8(ref a)  => ImageLuma8(colorops::grayscale(a)),
                        ImageLumaA8(ref a) => ImageLumaA8(colorops::grayscale(a)),
                        ImageRgb8(ref a)   => ImageRgb8(colorops::grayscale(a)),
                        ImageRgba8(ref a)  => ImageRgba8(colorops::grayscale(a)),
                }
        }*/

        /// Invert the colors of this image.
        /// This method operates inplace.
        pub fn invert(&mut self) {
                match *self {
                        ImageLuma8(ref mut p)  => colorops::invert(p),
                        ImageLumaA8(ref mut p) => colorops::invert(p),
                        ImageRgb8(ref mut p)   => colorops::invert(p),
                        ImageRgba8(ref mut p)  => colorops::invert(p),
                }
        }

        /// Resize this image using the specified filter algorithm.
        /// Returns a new image. The image's aspect ratio is preserved.
        ///```nwidth``` and ```nheight``` are the new image's dimensions
        pub fn resize(&self, nwidth: u32, nheight: u32, filter: sample::FilterType) -> Image {
                let (width, height) = self.dimensions();

                let ratio  = width as f32 / height as f32;
                let nratio = nwidth as f32 / nheight as f32;

                let scale = if nratio > ratio {
                        nheight as f32 / height as f32
                } else {
                        nwidth as f32 / width as f32
                };

                let width2  = (width as f32 * scale) as u32;
                let height2 = (height as f32 * scale) as u32;

                self.resize_exact(width2, height2, filter)
        }

        /// Resize this image using the specified filter algorithm.
        /// Returns a new image. Does not preserve aspect ratio.
        ///```nwidth``` and ```nheight``` are the new image's dimensions
        pub fn resize_exact(&self, nwidth: u32, nheight: u32, filter: sample::FilterType) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(sample::resize(p, nwidth, nheight, filter)),
                        ImageLumaA8(ref p) => ImageLumaA8(sample::resize(p, nwidth, nheight, filter)),
                        ImageRgb8(ref p)   => ImageRgb8(sample::resize(p, nwidth, nheight, filter)),
                        ImageRgba8(ref p)  => ImageRgba8(sample::resize(p, nwidth, nheight, filter)),
                }
        }

        /// Perfomrs a Gausian blur on this image.
        /// ```sigma``` is a meausure of how much to blur by.
        pub fn blur(&self, sigma: f32) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(sample::blur(p, sigma)),
                        ImageLumaA8(ref p) => ImageLumaA8(sample::blur(p, sigma)),
                        ImageRgb8(ref p)   => ImageRgb8(sample::blur(p, sigma)),
                        ImageRgba8(ref p)  => ImageRgba8(sample::blur(p, sigma)),
                }
        }

        /// Performs an unsharpen mask on ```pixels```
        /// ```sigma``` is the amount to blur the image by.
        /// ```threshold``` is a control of how much to sharpen.
        /// see https://en.wikipedia.org/wiki/Unsharp_masking#Digital_unsharp_masking
        pub fn unsharpen(&self, sigma: f32, threshold: i32) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(sample::unsharpen(p, sigma, threshold)),
                        ImageLumaA8(ref p) => ImageLumaA8(sample::unsharpen(p, sigma, threshold)),
                        ImageRgb8(ref p)   => ImageRgb8(sample::unsharpen(p, sigma, threshold)),
                        ImageRgba8(ref p)  => ImageRgba8(sample::unsharpen(p, sigma, threshold)),
                }
        }

        /// Filters this image with the specified 3x3 kernel.
        pub fn filter3x3(&self, kernel: &[f32]) -> Image {
                if kernel.len() != 9 {
                        return self.clone()
                }

                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(sample::filter3x3(p, kernel)),
                        ImageLumaA8(ref p) => ImageLumaA8(sample::filter3x3(p, kernel)),
                        ImageRgb8(ref p)   => ImageRgb8(sample::filter3x3(p, kernel)),
                        ImageRgba8(ref p)  => ImageRgba8(sample::filter3x3(p, kernel)),
                }
        }

        /// Adjust the contrast of ```pixels```
        /// ```contrast``` is the amount to adjust the contrast by.
        /// Negative values decrease the constrast and positive values increase the constrast.
        pub fn adjust_contrast(&self, c: f32) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(colorops::contrast(p, c)),
                        ImageLumaA8(ref p) => ImageLumaA8(colorops::contrast(p, c)),
                        ImageRgb8(ref p)   => ImageRgb8(colorops::contrast(p, c)),
                        ImageRgba8(ref p)  => ImageRgba8(colorops::contrast(p, c)),
                }
        }

        /// Brighten ```pixels```
        /// ```value``` is the amount to brighten each pixel by.
        /// Negative values decrease the brightness and positive values increase it.
        pub fn brighten(&self, value: i32) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(colorops::brighten(p, value)),
                        ImageLumaA8(ref p) => ImageLumaA8(colorops::brighten(p, value)),
                        ImageRgb8(ref p)   => ImageRgb8(colorops::brighten(p, value)),
                        ImageRgba8(ref p)  => ImageRgba8(colorops::brighten(p, value)),
                }
        }

        ///Flip this image vertically
        pub fn flipv(&self) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(affine::flip_vertical(p)),
                        ImageLumaA8(ref p) => ImageLumaA8(affine::flip_vertical(p)),
                        ImageRgb8(ref p)   => ImageRgb8(affine::flip_vertical(p)),
                        ImageRgba8(ref p)  => ImageRgba8(affine::flip_vertical(p)),
                }
        }

        ///Flip this image horizontally
        pub fn fliph(&self) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(affine::flip_horizontal(p)),
                        ImageLumaA8(ref p) => ImageLumaA8(affine::flip_horizontal(p)),
                        ImageRgb8(ref p)   => ImageRgb8(affine::flip_horizontal(p)),
                        ImageRgba8(ref p)  => ImageRgba8(affine::flip_horizontal(p)),
                }
        }

        ///Rotate this image 90 degrees clockwise.
        pub fn rotate90(&self) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(affine::rotate90(p)),
                        ImageLumaA8(ref p) => ImageLumaA8(affine::rotate90(p)),
                        ImageRgb8(ref p)   => ImageRgb8(affine::rotate90(p)),
                        ImageRgba8(ref p)  => ImageRgba8(affine::rotate90(p)),
                }
        }

        ///Rotate this image 180 degrees clockwise.
        pub fn rotate180(&self) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(affine::rotate180(p)),
                        ImageLumaA8(ref p) => ImageLumaA8(affine::rotate180(p)),
                        ImageRgb8(ref p)   => ImageRgb8(affine::rotate180(p)),
                        ImageRgba8(ref p)  => ImageRgba8(affine::rotate180(p)),
                }
        }

        ///Rotate this image 270 degrees clockwise.
        pub fn rotate270(&self) -> Image {
                match *self {
                        ImageLuma8(ref p)  => ImageLuma8(affine::rotate270(p)),
                        ImageLumaA8(ref p) => ImageLumaA8(affine::rotate270(p)),
                        ImageRgb8(ref p)   => ImageRgb8(affine::rotate270(p)),
                        ImageRgba8(ref p)  => ImageRgba8(affine::rotate270(p)),
                }
        }
}

impl<T: Primitive, P: Pixel<T> + Clone + Copy> GenericImage<P> for ImageBuf<P> {
        fn from_pixel(width: u32, height: u32, pixel: P) -> ImageBuf<P> {
                let buf = Vec::from_elem(width as uint * height as uint, pixel.clone());

                ImageBuf::new(buf, width, height)
        }

        fn dimensions(&self) -> (u32, u32) {
                (self.width, self.height)
        }

        fn bounds(&self) -> (u32, u32, u32, u32) {
                (self.xoffset, self.yoffset, self.width, self.height)
        }

        fn get_pixel(&self, x: u32, y: u32) -> P {
                let y0  = self.yoffset as uint;
                let x0  = self.xoffset as uint;
                let y   = y as uint;
                let x   = x as uint;
                let stride = self.vstride as uint;
                let buf = self.pixels.as_slice();

                buf[(y0 + y) * stride + x0 + x]
        }

        fn put_pixel(&mut self, x: u32, y: u32, pixel: P) {
                let y0  = self.yoffset as uint;
                let x0  = self.xoffset as uint;
                let y   = y as uint;
                let x   = x as uint;
                let stride = self.vstride as uint;
                let buf = self.pixels.as_mut_slice();

                buf[(y0 + y) * stride + x0 + x] = pixel;
        }
}

fn decoder_to_image<I: ImageDecoder>(codec: I) -> ImageResult<Image> {
	let mut codec = codec;

	let color  = try!(codec.colortype());
	let buf    = try!(codec.read_image());
	let (w, h) = try!(codec.dimensions());

	let image = match color {
		colortype::RGB(8) => {
                        let p = buf.as_slice()
                                   .chunks(3)
                                   .map(|a| pixel::Rgb::<u8>(a[0], a[1], a[2]))
                                   .collect();

                        ImageRgb8(ImageBuf::new(p, w, h))
                }

                colortype::RGBA(8) => {
                        let p = buf.as_slice()
                                   .chunks(4)
                                   .map(|a| pixel::Rgba::<u8>(a[0], a[1], a[2], a[3]))
                                   .collect();

                        ImageRgba8(ImageBuf::new(p, w, h))
                }

                colortype::Grey(8) => {
                        let p = buf.as_slice()
                                   .iter()
                                   .map(|a| pixel::Luma::<u8>(*a))
                                   .collect();

                        ImageLuma8(ImageBuf::new(p, w, h))
                }

                colortype::GreyA(8) => {
                        let p = buf.as_slice()
                                   .chunks(2)
                                   .map(|a| pixel::LumaA::<u8>(a[0], a[1]))
                                   .collect();

                        ImageLumaA8(ImageBuf::new(p, w, h))
                }

                _ => return Err(UnsupportedColor)
	};

	Ok(image)
}

fn image_to_bytes(image: &Image) -> Vec<u8> {
        let mut r = Vec::new();

        match *image {
                //TODO: consider transmuting
                ImageLuma8(ref a) => {
                        for &i in a.pixels().iter() {
                                r.push(i.channel());
                        }
                }

                ImageLumaA8(ref a) => {
                        for &i in a.pixels().iter() {
                                let (l, a) = i.channels();
                                r.push(l);
                                r.push(a);
                        }
                }

                ImageRgb8(ref a)  => {
                        for &i in a.pixels().iter() {
                                let (red, g, b) = i.channels();
                                r.push(red);
                                r.push(g);
                                r.push(b);
                        }
                }

                ImageRgba8(ref a) => {
                        for &i in a.pixels().iter() {
                                let (red, g, b, alpha) = i.channels();
                                r.push(red);
                                r.push(g);
                                r.push(b);
                                r.push(alpha);
                        }
                }
        }

        r
}