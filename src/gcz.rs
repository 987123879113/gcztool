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

fn rgba555_to_rgba(format: &SourcePlatform, buf: &[u8]) -> Vec<u8> {
    let mut output = Vec::<u8>::new();

    for i in (0..buf.len()).step_by(2) {
        let rgb555 = match format {
            SourcePlatform::Firebeat => ((buf[i] as u16) << 8) | (buf[i + 1] as u16),
            SourcePlatform::Python | SourcePlatform::PC => ((buf[i + 1] as u16) << 8) | (buf[i] as u16),
        };

        let (r, g, b, a) = match format {
            SourcePlatform::Firebeat => (
                ((rgb555 >> 10) & 0x1f) as u8,
                ((rgb555 >> 5) & 0x1f) as u8,
                ((rgb555 >> 0) & 0x1f) as u8,
                ((rgb555 >> 15) & 1) as u8,
            ),
            SourcePlatform::Python | SourcePlatform::PC => (
                ((rgb555 >> 0) & 0x1f) as u8,
                ((rgb555 >> 5) & 0x1f) as u8,
                ((rgb555 >> 10) & 0x1f) as u8,
                ((rgb555 >> 15) & 1) as u8,
            ),
        };

        output.push((r << 3) | (r >> 2));
        output.push((g << 3) | (g >> 2));
        output.push((b << 3) | (b >> 2));
        output.push(a * 0xff);
    }

    output.to_vec()
}

fn bgra555_to_rgba(format: &SourcePlatform, buf: &[u8]) -> Vec<u8> {
    let mut output = Vec::<u8>::new();

    for i in (0..buf.len()).step_by(2) {
        let rgb555 = match format {
            SourcePlatform::Firebeat => ((buf[i] as u16) << 8) | (buf[i + 1] as u16),
            SourcePlatform::Python | SourcePlatform::PC => ((buf[i + 1] as u16) << 8) | (buf[i] as u16),
        };

        let (r, g, b, a) = match format {
            SourcePlatform::Firebeat => (
                ((rgb555 >> 0) & 0x1f) as u8,
                ((rgb555 >> 5) & 0x1f) as u8,
                ((rgb555 >> 10) & 0x1f) as u8,
                ((rgb555 >> 15) & 1) as u8,
            ),
            SourcePlatform::Python | SourcePlatform::PC => (
                ((rgb555 >> 10) & 0x1f) as u8,
                ((rgb555 >> 5) & 0x1f) as u8,
                ((rgb555 >> 0) & 0x1f) as u8,
                ((rgb555 >> 15) & 1) as u8,
            ),
        };

        output.push((r << 3) | (r >> 2));
        output.push((g << 3) | (g >> 2));
        output.push((b << 3) | (b >> 2));
        output.push(a * 0xff);
    }

    output.to_vec()
}

pub fn load_texture_from_file(format: &SourcePlatform, filename: &OsStr) -> RgbaImage {
    let buf = fs::read(filename).expect("Could not read input GCZ file");
    let decomp = gcz_decompress(&buf);
    load_texture_from_memory(format, &decomp)
}

pub fn load_texture_from_memory(format: &SourcePlatform, buf: &[u8]) -> RgbaImage {
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

    let texture = RgbaImage::from_raw(
        img_w,
        img_h,
        if buf[0x02] == 0x20 {
            bgra555_to_rgba(format, &buf[0x18..0x18 + raw_bytes_size])
        } else {
            rgba555_to_rgba(format, &buf[0x18..0x18 + raw_bytes_size])
        },
    )
    .unwrap();

    if img_x != 0 || img_y != 0 {
        texture
    } else {
        texture
    }
}
