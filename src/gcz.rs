use super::charlib::SourcePlatform;
use image::RgbaImage;
use std::{ffi::OsStr, fs};

pub fn gcz_decompress(buf: &Vec<u8>) -> Vec<u8> {
    // Based on code from Keyboardmania 3rd
    let expected_size = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
    let mut out = vec![0; expected_size + 0x1000];
    let mut idx = 4;
    let mut out_idx = 0x1000;
    let mut window_idx = 0xfee;
    let mut flags = 0;

    while out_idx - 0x1000 < expected_size {
        flags >>= 1;

        if (flags & 0x100) == 0 {
            if idx >= buf.len() {
                break;
            }

            flags = 0xff00 | (buf[idx] as u16);
            idx += 1;
        }

        if (flags & 1) == 0 {
            let offset = {
                let mut val =
                    i16::from_le_bytes([buf[idx], (buf[idx + 1] & 0xf0) >> 4]) as i32 - window_idx;

                if val >= 0 {
                    val -= 0x1000
                }

                out_idx as i32 + val
            };
            let len = (buf[idx + 1] & 0xf) + 3;

            for i in 0..len {
                out[out_idx] = out[(offset + i as i32) as usize];
                out_idx += 1;
            }

            idx += 2;
            window_idx = (window_idx + len as i32) & 0xfff;
        } else {
            out[out_idx] = buf[idx];
            idx += 1;
            out_idx += 1;
            window_idx = (window_idx + 1) & 0xfff;
        }
    }

    out[0x1000..].to_vec()
}

fn raw_to_rgba(
    format: &SourcePlatform,
    buf: &[u8],
    r_mask: u16,
    g_mask: u16,
    b_mask: u16,
    a_mask: u16,
) -> Vec<u8> {
    let mut output = Vec::<u8>::new();

    for i in (0..buf.len()).step_by(2) {
        let pix = match format {
            SourcePlatform::Firebeat => ((buf[i] as u16) << 8) | (buf[i + 1] as u16),
            SourcePlatform::Python | SourcePlatform::PC => {
                ((buf[i + 1] as u16) << 8) | (buf[i] as u16)
            }
        };

        let r = {
            let val = (pix & r_mask) >> r_mask.trailing_zeros();
            ((val << 3) | (val >> 2)) as u8
        };
        let g = {
            let val = (pix & g_mask) >> g_mask.trailing_zeros();
            ((val << 3) | (val >> 2)) as u8
        };
        let b = {
            let val = (pix & b_mask) >> b_mask.trailing_zeros();
            ((val << 3) | (val >> 2)) as u8
        };
        let a = if (pix & a_mask) != 0 { 0xff } else { 0x00 };

        match format {
            SourcePlatform::Firebeat => {
                output.push(r);
                output.push(g);
                output.push(b);
            }
            SourcePlatform::Python | SourcePlatform::PC => {
                output.push(b);
                output.push(g);
                output.push(r);
            }
        };

        output.push(a);
    }

    output.to_vec()
}

pub fn load_texture_from_file(format: &SourcePlatform, filename: &OsStr) -> RgbaImage {
    let buf = fs::read(filename).expect("Could not read input GCZ file");
    let decomp = gcz_decompress(&buf);
    load_texture_from_memory(format, &decomp)
}

pub fn load_texture_from_memory(format: &SourcePlatform, buf: &[u8]) -> RgbaImage {
    if &buf[0..4] == b"DDS " {
        return load_dds_texture_from_memory(format, buf);
    } else if &buf[0..2] == b"GC" {
        return load_gc_texture_from_memory(format, buf);
    } else {
        panic!("Unknown texture format!");
    }
}

fn load_dds_texture_from_memory(format: &SourcePlatform, buf: &[u8]) -> RgbaImage {
    if (buf[0x50] & 0x40) == 0 {
        panic!("Don't know how to parse non-RGBA DDS files");
    }

    let img_w = u32::from_le_bytes(buf[0x0c..0x10].try_into().unwrap());
    let img_h = u32::from_le_bytes(buf[0x10..0x14].try_into().unwrap());
    let r_mask = u16::from_le_bytes(buf[0x58..0x5a].try_into().unwrap());
    let g_mask = u16::from_le_bytes(buf[0x5c..0x5e].try_into().unwrap());
    let b_mask = u16::from_le_bytes(buf[0x60..0x62].try_into().unwrap());
    let a_mask = u16::from_le_bytes(buf[0x64..0x66].try_into().unwrap());

    return RgbaImage::from_raw(
        img_w,
        img_h,
        raw_to_rgba(format, &buf[0x80..], r_mask, g_mask, b_mask, a_mask),
    )
    .unwrap();
}

fn load_gc_texture_from_memory(format: &SourcePlatform, buf: &[u8]) -> RgbaImage {
    let img_x = u16::from_be_bytes(buf[0x08..0x0a].try_into().unwrap()) as u32;
    let img_y = u16::from_be_bytes(buf[0x0a..0x0c].try_into().unwrap()) as u32;
    let img_w = u16::from_be_bytes(buf[0x0c..0x0e].try_into().unwrap()) as u32;
    let img_h = u16::from_be_bytes(buf[0x0e..0x10].try_into().unwrap()) as u32;
    let img_format = buf[0x13];
    let raw_bytes_size = (img_w * img_h * 2) as usize;

    // If any of these cases are hit then I need to add additional support
    assert!(img_x == 0, "Found GCZ texture with non-0 X value");
    assert!(img_y == 0, "Found GCZ texture with non-0 Y value");
    // If this is non-0 then it might be an 8-bit texture (img_format == 0x08)
    assert!(img_format == 0x00, "Found GCZ texture with unknown format");

    let (r_mask, g_mask, b_mask, a_mask) = if buf[0x02] == 0x20 {
        (0x1f, 0x3e0, 0x7c00, 0x8000)
    } else {
        (0x7c00, 0x3e0, 0x1f, 0x8000)
    };

    let texture = RgbaImage::from_raw(
        img_w,
        img_h,
        raw_to_rgba(
            format,
            &buf[0x18..0x18 + raw_bytes_size],
            r_mask,
            g_mask,
            b_mask,
            a_mask,
        ),
    )
    .unwrap();

    if img_x != 0 || img_y != 0 {
        texture
    } else {
        texture
    }
}
