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