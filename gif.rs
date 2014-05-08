use std::os;
use std::io::File;
use std::io::IoResult;

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

    fn read_logical_screen_descriptor(&mut self) -> IoResult<()> {
        let width  = try!(self.r.read_le_u16());
        let height = try!(self.r.read_le_u16());
        let fields = try!(self.r.read_u8());
        println!("width {0} height {1}", width, height);
        println!("packed: fields {:0<8t}", fields);

        let global_table = fields & 0x80 != 0;
        println!("global color table {}", global_table);

        let color_resolution = ((fields & 70) >> 4) + 1;
        println!("color resolution {}", color_resolution);

        let sorted = fields & 8 != 0;
        println!("table sorted {}", sorted);

        if global_table {
            let table_size = 1 << ((fields & 7) + 1);
            println!("table size {} bytes", 3 * table_size);
        }

        if global_table {
            let bgrnd_index = try!(self.r.read_u8());
            println!("color index {}", bgrnd_index);
        }

        let aspect_ratio = try!(self.r.read_u8());
        println!("aspect ratio {}", (aspect_ratio + 15) / 64);

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