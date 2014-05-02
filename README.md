##Rust image-formats
Learning Rust; Implementing image formats

#TODO
+ Implement Pure rust deflate compression
+ Implement Basic jpeg encoder
+ Implement GIF Decoder
+ Some sort of error handling instead of failure

#BUGS
+ idct is slow
+ JPEG decoder makes wrong assumptions about MCU size
+ Paletted images < 8 bits per pixel not handled properly
