#Rust image-formats
Learning Rust; Implementing image formats

###TODO
+ Implement Pure rust deflate compression
+ Encoding 16 bit images to jpeg
+ Decoding interlaced jpeg and png and gif
+ ~~Decoding webp images~~(luma only)
+ Precalculate filters once per row and column.
+ Change lzw and deflate to be lazy.

###BUGS
+ Paletted images < 8 bits per pixel not handled properly


#I'm developing an alternate library at https://github.com/PistonDevelopers/rust-image