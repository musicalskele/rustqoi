const QOI_OP_INDEX   : u8 = 0b00000000;
const QOI_OP_DIFF	 : u8 = 0b01000000;
const QOI_OP_LUMA	 : u8 = 0b10000000;
const QOI_OP_RUN     : u8 = 0b11000000;
const QOI_CHUNK_MASK : u8 = 0b11000000;
const QOI_OP_RGB     : u8 = 0b11111110;
const QOI_OP_RGBA    : u8 = 0b11111111;
const QOI_PADDING    :[u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
const QOI_MAGIC_RAW  :[u8; 4] = *b"qoif";
const QOI_PADDING_LEN: u8 = QOI_PADDING.len() as u8;
//const QOI_MAGIC_U32: u32 = u32::from_be_bytes(QOI_MAGIC_RAW);
//const QOI_MAGIC_LEN: u8 = QOI_MAGIC_RAW.len() as u8;
const QOI_HEADER_SIZE: u8 = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channels {
    RGB  = 3,
    RGBA = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Colorspace {
    Linear = 0,
    SRGB   = 1,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QoiHeader {
    pub width      : u32,
    pub height     : u32,
    pub channels   : Channels,
    pub colorspace : Colorspace,
}

impl QoiHeader {
    pub const MAGIC: [u8; 4] = QOI_MAGIC_RAW;
    pub fn new(width: u32, height: u32, channels: Channels, colorspace: Colorspace) -> Self {
        Self {
            width,
            height,
            channels,
            colorspace,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct QoiPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl QoiPixel {
    fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// takes in Qoipixel, applies (3r + 5g + 7b + 11a) % 64, returns that as the index position for that color
const fn qoi_color_hash(pixel: QoiPixel) -> u8 {
    let v = u32::from_ne_bytes([pixel.r, pixel.g, pixel.b, pixel.a]);
    let s = (((v as u64) << 32) | (v as u64)) & 0xFF00FF0000FF00FF;
    s.wrapping_mul(0x030007000005000Bu64.to_le()).swap_bytes() as u8 & 63
}

pub fn qoi_encode(pixels: &[u8], qoi_header: &QoiHeader) -> Vec<u8> {
    let max_size = QOI_PADDING_LEN as usize
        + QOI_HEADER_SIZE as usize
        + qoi_header.width as usize
        * qoi_header.height as usize
        *(qoi_header.channels as usize + 1);
    let mut bytebuffer: Vec<u8> = Vec::with_capacity(max_size);
    bytebuffer.extend_from_slice(&QoiHeader::MAGIC);
    bytebuffer.extend_from_slice(&qoi_header.width.to_be_bytes());
    bytebuffer.extend_from_slice(&qoi_header.height.to_be_bytes());
    bytebuffer.push(qoi_header.channels as u8);
    bytebuffer.push(qoi_header.colorspace as u8);

    let mut pixel_index = [QoiPixel::new(0x00, 0x00, 0x00, 0x00); 64];
    let mut last_pixel: QoiPixel = QoiPixel::new(0x00, 0x00, 0x00, 0xFF);
    let end_pixel = (pixels.len() / qoi_header.channels as usize) - 1;
    let mut iter_index: usize = 0;
    let mut run: usize = 0;
    let mut bytes_iter = pixels.iter();

    while let Some(&byte) = bytes_iter.next() {
        let cur_pixel = QoiPixel::new(
            byte,
            *bytes_iter.next().unwrap(),
            *bytes_iter.next().unwrap(),
            match qoi_header.channels as u8 {
                4 => *bytes_iter.next().unwrap(),
                _ => 0xFF,
            },
        );

        match cur_pixel == last_pixel {
            true => {
                run += 1;
                if run == 62 || iter_index == end_pixel {
                    bytebuffer.push(QOI_OP_RUN | (run - 1) as u8);
                    run = 0;
                }
            }
            false => {
                if run > 0 {
                    bytebuffer.push(QOI_OP_RUN | (run - 1) as u8);
                    run = 0;
                }

                let pixel_hash = qoi_color_hash(cur_pixel) as usize;
                
                match pixel_index[pixel_hash] == cur_pixel {
                    true => bytebuffer.push(pixel_hash as u8),
                    false => {
                        pixel_index[pixel_hash] = cur_pixel;
                        match cur_pixel.a == last_pixel.a {
                            true => {
                                let dr = (cur_pixel.r.wrapping_sub(last_pixel.r)) as i8;
                                let dg = (cur_pixel.g.wrapping_sub(last_pixel.g)) as i8;
                                let db = (cur_pixel.b.wrapping_sub(last_pixel.b)) as i8;
                                let dg_r = dr.wrapping_sub(dg);
                                let dg_b = db.wrapping_sub(dg);
                                match (dr   > -3 && dr   < 2 && dg >  -3 && dg <  2 && db >   -3 && db   < 2,
                                       dg_r > -9 && dg_r < 8 && dg > -33 && dg < 32 && dg_b > -9 && dg_b < 8) {
                                 (true, _) => {
                                     let dr = dr as u8;
                                     let dg = dg as u8;
                                     let db = db as u8;
                                     bytebuffer.push(QOI_OP_DIFF << 0 
                                         | (dr.wrapping_add(2)) << 4 
                                         | (dg.wrapping_add(2)) << 2 
                                         | (db.wrapping_add(2)));
                                 },
                                 (false, true) => {
                                     let dg_r = (dg_r + 8) as u8;
                                     let dg_b = (dg_b + 8) as u8;
                                     let dg   = (dg  + 32) as u8;
                                     bytebuffer.push(QOI_OP_LUMA << 0 | dg);
                                     bytebuffer.push(dg_r << 4 | dg_b);
                                 },
                                 (false, false) => {
                                     bytebuffer.extend_from_slice(&[
                                         QOI_OP_RGB,
                                         cur_pixel.r,
                                         cur_pixel.g,
                                         cur_pixel.b
                                     ]);
                                 }
                             }
                            }
                            false => bytebuffer.extend_from_slice(&[
                                QOI_OP_RGBA,
                                cur_pixel.r,
                                cur_pixel.g,
                                cur_pixel.b,
                                cur_pixel.a,
                            ]),
                        }
                    }
                }
            }
        }
        
        last_pixel = cur_pixel;
        iter_index += 1;
    }

    bytebuffer.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    bytebuffer
}

pub fn qoi_decode(data: &[u8]) -> (Vec<u8>, QoiHeader) {

	let width = u32::from_be_bytes(data[4..8].try_into().unwrap());
    let height = u32::from_be_bytes(data[8..12].try_into().unwrap());
	let channels = match data[12] {
		3 => Channels::RGB,
		4 => Channels::RGBA,
		_ => {panic!("invalid color channel")}
	};
	let colorspace: Colorspace = match data[13] {
		0 => Colorspace::SRGB,
		1 => Colorspace::Linear,
		_ => {panic!("invalid colorspace")}
	};

	// header handling
	let desc = QoiHeader {
		width,
		height,
		channels,
		colorspace,
	};

	// pixel handling
	let pixel_len = desc.width * desc.height * (channels as u32);
    let mut pixels:Vec<u8> = Vec::with_capacity(pixel_len as usize);

	let mut iter = data[14..data.len() - 8].iter();

    let mut index = [QoiPixel::new(0, 0, 0, 0); 64];
    let mut last_pixel = QoiPixel::new(0, 0, 0, 255);

	// main decoding loop
	while let Some(&byte) = iter.next() {
		match byte{
			QOI_OP_RGB 	=>  {
				last_pixel.r = *iter.next().unwrap();
				last_pixel.g = *iter.next().unwrap();
				last_pixel.b = *iter.next().unwrap();
				index[qoi_color_hash(last_pixel) as usize] = last_pixel;
				pixels.extend([last_pixel.r,last_pixel.g,last_pixel.b]);
				if channels as u8 == 4 {pixels.push(last_pixel.a);};
				continue
			},
			QOI_OP_RGBA => 	{
				last_pixel.r = *iter.next().unwrap();
				last_pixel.g = *iter.next().unwrap();
				last_pixel.b = *iter.next().unwrap();
				last_pixel.a = *iter.next().unwrap();
				index[qoi_color_hash(last_pixel) as usize] = last_pixel;
				pixels.extend([last_pixel.r,last_pixel.g,last_pixel.b,last_pixel.a]);
				continue
			},
			_			=>	{}	 // idk if its possible to merge these match statements
		} 
		match byte & QOI_CHUNK_MASK {
			QOI_OP_RUN => {
				let runlen = (byte & 0b00111111) + 1;
				let items = match channels as u8 {
					4 => vec![last_pixel.r, last_pixel.g, last_pixel.b, last_pixel.a],
					_ => vec![last_pixel.r, last_pixel.g, last_pixel.b],
				};
				for _ in 0..runlen {pixels.extend(items.iter());}

				continue
			},
			QOI_OP_INDEX => 	{
				last_pixel = index[byte as usize];
				match channels as u8 {
					4 => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b, &last_pixel.a]),
					_ => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b]),
				}
				
				continue
			},
			QOI_OP_DIFF =>  {
				let delta_r = ((byte & 0b00110000) >> 4) as i8 - 2;
                let delta_g = ((byte & 0b00001100) >> 2) as i8 - 2;
                let delta_b = ((byte & 0b00000011)     ) as i8 - 2;

				last_pixel.r = last_pixel.r.wrapping_add_signed(delta_r);
                last_pixel.g = last_pixel.g.wrapping_add_signed(delta_g);
                last_pixel.b = last_pixel.b.wrapping_add_signed(delta_b);
				
				index[qoi_color_hash(last_pixel) as usize] = last_pixel;
				match channels as u8 {
					4 => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b, &last_pixel.a]),
					_ => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b]),
				}

				continue
			},
			QOI_OP_LUMA => 	{
				let byte0 	= &byte;
				let byte1 	= iter.next().unwrap();
				let delta_g = ( byte0 & 0b00111111		) as i8 - 32;
                let df_red  = ((byte1 & 0b11110000) >> 4) as i8 - 8;
                let df_blue = ( byte1 & 0b00001111		) as i8 - 8; 

				last_pixel.r = last_pixel.r.wrapping_add_signed(delta_g).wrapping_add_signed(df_red);
				last_pixel.g = last_pixel.g.wrapping_add_signed(delta_g);
				last_pixel.b = last_pixel.b.wrapping_add_signed(delta_g).wrapping_add_signed(df_blue);
				
				index[qoi_color_hash(last_pixel) as usize] = last_pixel;
				match channels as u8 {
					4 => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b, &last_pixel.a]),
					_ => pixels.extend([&last_pixel.r, &last_pixel.g, &last_pixel.b]),
				}				
				continue
			},
			_	        =>	{unreachable!()}
		}
        
	}
	(pixels,desc)
}