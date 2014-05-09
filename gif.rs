use std::os;
use std::io::File;
use std::io::IoResult;

static IMAGEDESCRIPTOR: u8 = 0x2C;
static EXTENSION: u8 = 0x21;
static APPLICATION: u8 = 0xFF;
static GRAPHICCONTROL: u8 = 0xF9;

struct GIFDecoder <R> {
    r: R,

    width: u16,
    height: u16,
}

impl<R: Reader> GIFDecoder<R> {
    pub fn new(r: R) -> GIFDecoder<R> {
        GIFDecoder {
            r: r,

            width: 0,
            height: 0
        }
    }

    fn read_header(&mut self) -> IoResult<()> {
        let signature = try!(self.r.read_exact(3));
        let version   = try!(self.r.read_exact(3));

        assert!(signature.as_slice() == "GIF".as_bytes());
        assert!(version.as_slice() == "87a".as_bytes() ||
                version.as_slice() == "89a".as_bytes());

        Ok(())
    }

    fn read_block(&mut self) -> IoResult<~[u8]> {
        let size = try!(self.r.read_u8());

        self.r.read_exact(size as uint)
    }

    fn read_image_descriptor(&mut self) -> IoResult<()> {
        println!("\nIMAGE DESCRIPTOR");

        let image_left   = try!(self.r.read_le_u16());
        let image_top    = try!(self.r.read_le_u16());
        let image_width  = try!(self.r.read_le_u16());
        let image_height = try!(self.r.read_le_u16());

        let fields = try!(self.r.read_u8());

        let local_table = fields & 80 != 0;
        let interlace   = fields & 40 != 0;
        let sorted      = fields & 20 != 0;
        let table_size  = fields & 7;

        println!("image top {0} left {1}", image_top, image_left);
        println!("image width {0} height {1}", image_width, image_height);
        println!("local table {}", local_table);
        println!("interlace {}", interlace);
        println!("sorted {}", sorted);
        println!("table size {}", 1 << (table_size + 1));

        let minimum_code_size = try!(self.r.read_u8());

        loop {
            let b = try!(self.read_block());
            if b.len() == 0 {
                break
            }
        }

        Ok(())
    }

    fn read_graphic_control_extension(&mut self) -> IoResult<()> {
        println!("GRAPHIC CONTROL EXTENSION");

        let size   = try!(self.r.read_u8());
        let fields = try!(self.r.read_u8());
        let delay  = try!(self.r.read_le_u16());
        let trans  = try!(self.r.read_u8());

        println!("size {}", size);
        println!("fields {:0>8t}", fields);
        println!("delay {}ms", delay * 10);

        let term = try!(self.r.read_u8());
        println!("terminator 0x{:X}", term);

        Ok(())
    }

    fn read_logical_screen_descriptor(&mut self) -> IoResult<()> {
        let width  = try!(self.r.read_le_u16());
        let height = try!(self.r.read_le_u16());
        let fields = try!(self.r.read_u8());
        println!("width {0} height {1}", width, height);
        println!("packed: fields {:0<8t}", fields);

        let global_table = fields & 0x80 != 0;
        println!("global color table {}", global_table);

        let color_resolution = ((fields & 70) >> 4) + 1;
        println!("expected color resolution {}", 1 << color_resolution);

        let sorted = fields & 8 != 0;
        println!("table sorted {}", sorted);

        let entries = if global_table {
            1 << ((fields & 7) + 1)
        } else {
            0
        };

        println!("entries in table {}", entries);

        if global_table {
            let bgrnd_index = try!(self.r.read_u8());
            println!("color index {}", bgrnd_index);
        }

        let aspect_ratio = try!(self.r.read_u8());
        println!("aspect ratio {}", (aspect_ratio + 15) / 64);

        let global_color_table = if global_table {
            try!(self.r.read_exact(3 * entries))
        } else {
            ~[]
        };

        println!("colors");
        for (i, rgb) in global_color_table.chunks(3).enumerate() {
            println!("\t{3:3} -> red {0:3} green {1:3} blue {2}", rgb[0], rgb[1], rgb[2], i);
        }

        loop {
            let byte = try!(self.r.read_u8());

            println!("byte 0x{:X}", byte);

            if byte == EXTENSION {
                let e = try!(self.r.read_u8());
                println!("extension identifier 0x{:X}", e);

                if e == GRAPHICCONTROL {
                    let _ = try!(self.read_graphic_control_extension());
                }
                else if e == APPLICATION {
                    let size = try!(self.r.read_u8());
                    let _ = try!(self.r.read_exact(size as uint));

                    loop {
                        let b = try!(self.read_block());
                        if b.len() == 0 {
                            break
                        }
                    }
                }


            } else if byte == IMAGEDESCRIPTOR {
                let _ = try!(self.read_image_descriptor());
            } else {
                break
            }
        }

        Ok(())
    }

    pub fn decode(&mut self) -> IoResult<()> {
        let _ = try!(self.read_header());
        let _ = try!(self.read_logical_screen_descriptor());

        Ok(())
    }
}

fn main() {
    let file = if os::args().len() == 2 {
        os::args()[1]
    } else {
        fail!("provide a file")
    };

    let fin = File::open(&Path::new(file.clone())).unwrap();

    let mut g = GIFDecoder::new(fin);
    let _ = g.decode();
}