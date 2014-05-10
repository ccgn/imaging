use std::os;
use std::io::File;
use std::io::IoResult;

static IMAGEDESCRIPTOR: u8 = 0x2C;
static EXTENSION: u8 = 0x21;
static APPLICATION: u8 = 0xFF;
static GRAPHICCONTROL: u8 = 0xF9;
static TRAILER: u8 = 0x3B;

struct GIFDecoder <R> {
    r: R,

    width: u16,
    height: u16,

    global_table: [(u8, u8, u8), ..256],
    local_table: Option<~[u8]>,

    backgroud_index: u8,
}

impl<R: Reader> GIFDecoder<R> {
    pub fn new(r: R) -> GIFDecoder<R> {
        GIFDecoder {
            r: r,

            width: 0,
            height: 0,

            global_table: [(0u8, 0u8, 0u8), ..256],
            local_table: None,

            backgroud_index: 0,
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

    fn read_extension(&mut self) -> IoResult<()> {
        let identifier = try!(self.r.read_u8());

        match identifier {
            APPLICATION    => try!(self.read_application_extension()),
            GRAPHICCONTROL => try!(self.read_graphic_control_extension()),
            _ => fail!("unimplemented")
        }

        Ok(())
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

        let mut data = ~[];

        loop {
            let b = try!(self.read_block());
            if b.len() == 0 {
                break
            }

            data = data + b;
        }

        println!("data {}", data.len());
        Ok(())
    }

    fn read_graphic_control_extension(&mut self) -> IoResult<()> {
        println!("\nGRAPHIC CONTROL EXTENSION");

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

    fn read_application_extension(&mut self) -> IoResult<()> {
        let size = try!(self.r.read_u8());
        let _ = try!(self.r.read_exact(size as uint));

        loop {
            let b = try!(self.read_block());
            if b.len() == 0 {
                break
            }
        }

        Ok(())
    }

    fn read_logical_screen_descriptor(&mut self) -> IoResult<()> {
        self.width  = try!(self.r.read_le_u16());
        self.height = try!(self.r.read_le_u16());

        let fields = try!(self.r.read_u8());

        let global_table = fields & 0x80 != 0;
        let entries = if global_table { 1 << ((fields & 7) + 1)}
                      else {0};

        if global_table {
            self.backgroud_index = try!(self.r.read_u8());
        }

        let _aspect_ratio = try!(self.r.read_u8());

        let buf = try!(self.r.read_exact(3 * entries));

        for (i, rgb) in buf.chunks(3).enumerate() {
           self.global_table[i] = (rgb[0], rgb[1], rgb[2]);
        }

        Ok(())
    }

    pub fn decode(&mut self) -> IoResult<()> {
        let _ = try!(self.read_header());
        let _ = try!(self.read_logical_screen_descriptor());

        loop {
            let byte = try!(self.r.read_u8());

            println!("byte 0x{:X}", byte);

            if byte == EXTENSION {
                let _ = try!(self.read_extension());
            } else if byte == IMAGEDESCRIPTOR {
                let _ = try!(self.read_image_descriptor());
            } else {
                break
            }
        }

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