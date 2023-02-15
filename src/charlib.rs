use super::gcz::load_texture_from_file;
use image::{GenericImage, GenericImageView, RgbaImage};
use std::{path::Path, fs};

#[derive(PartialEq, Debug)]
pub enum SourcePlatform {
    Firebeat,
    Python,
    PC,
}

fn read_formatted_u32(format: &SourcePlatform, buf: &[u8], offset: usize) -> u32 {
    let bytes = buf[offset..offset + 4].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u32::from_be_bytes(bytes),
        SourcePlatform::Python => u32::from_le_bytes(bytes),
        SourcePlatform::PC => u32::from_le_bytes(bytes),
    }
}

fn read_formatted_u32_special(format: &SourcePlatform, buf: &[u8], offset: usize) -> u32 {
    let bytes = buf[offset..offset + 4].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u32::from_be_bytes(bytes),
        SourcePlatform::Python => u32::from_le_bytes(bytes),
        SourcePlatform::PC => u32::from_be_bytes(bytes),
    }
}

fn read_formatted_u16(format: &SourcePlatform, buf: &[u8], offset: usize) -> u16 {
    let bytes = buf[offset..offset + 2].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u16::from_be_bytes(bytes),
        SourcePlatform::Python => u16::from_le_bytes(bytes),
        SourcePlatform::PC => u16::from_le_bytes(bytes),
    }
}

fn generate_atlas(format: &SourcePlatform, chunk: &[u8], texture_path: &Path) -> RgbaImage {
    let image_count = read_formatted_u16(&format, chunk, 0x02);
    let mut textures = Vec::<RgbaImage>::new();
    let mut texture_w = 0;
    let mut texture_h = 0;

    for i in 0..image_count {
        let start_offset = (0x14 + (i * 0x20)) as usize;
        let end_offset = {
            let mut j = 0;

            for _ in 0..0x20 {
                if chunk[start_offset + j] == 0 {
                    break;
                }

                j += 1;
            }

            start_offset + j
        };

        let filename = &String::from_utf8(chunk[start_offset..end_offset].try_into().unwrap())
            .expect("Could not convert filename");
        let filename = texture_path.join(Path::new(filename).strip_prefix("/").unwrap());
        let tex = load_texture_from_file(&format, filename.as_os_str());

        if i == 0 {
            texture_w = tex.width();
        } else {
            assert!(texture_w == tex.width(), "Texture changes width?");
        }

        texture_h += tex.height();

        textures.push(tex);
    }

    let mut texture = RgbaImage::new(texture_w, texture_h);

    let mut cury = 0;
    for curtex in textures {
        texture
            .copy_from(&curtex, 0, cury)
            .expect("Could not paste sub texture to main texture");
        cury += curtex.height();
    }

    texture
}

// fn parse_idx(buf: Vec<u8>, texture_path: &Path, export: bool) {
//     let val1 = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
//     let val2 = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;

//     let index_format = if val1 > buf.len() && val2 > buf.len() {
//         SourcePlatform::Firebeat
//     } else if val1 > buf.len() && val2 <= buf.len() {
//         SourcePlatform::PC
//     } else {
//         SourcePlatform::Python
//     };

//     let mut offset = 0;
//     let mut chunk_id = 0;
//     while offset < buf.len() {
//         let chunk_size = read_formatted_u32_special(&index_format, &buf, offset) as usize;
//         offset += 4;

//         let chunk = &buf[offset..offset + chunk_size];

//         if chunk_id == 0 {
//             let sprites_table_offset = read_formatted_u32(&index_format, chunk, 0x04) as usize;
//             let sect2_offset = read_formatted_u32(&index_format, chunk, 0x08) as usize;
//             let sect3_offset = read_formatted_u32(&index_format, chunk, 0x0c) as usize;
//             let sect4_offset = read_formatted_u32(&index_format, chunk, 0x10) as usize;

//             let graphic_id = read_formatted_u16(&index_format, chunk, 0x00);

//             let atlas = generate_atlas(&index_format, chunk, texture_path);

//             // These are called "frames" in charlib
//             let sprites = {
//                 let mut chunks = Vec::<SubImage<&RgbaImage>>::new();
//                 let mut i = 0;
//                 let mut sprite_idx = 0;

//                 loop {
//                     let x =
//                         read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x00)
//                             as u32;
//                     let y =
//                         read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x02)
//                             as u32;
//                     let w =
//                         read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x04)
//                             as u32;
//                     let h =
//                         read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x06)
//                             as u32;
//                     i += 8;

//                     if w == 0 || h == 0 {
//                         break;
//                     }

//                     chunks.push(atlas.view(x, y, w, h));
//                     sprite_idx += 1;
//                 }

//                 chunks
//             };

//             // Something to do with animations or maybe layers?
//             let mut unk_blocks = Vec::<Vec<&[u8]>>::new();
//             let mut cur_blocks = Vec::<&[u8]>::new();
//             for offset in (sect4_offset..chunk.len()).step_by(0x24) {
//                 let block = &chunk[offset..offset + 0x24];

//                 cur_blocks.push(block);

//                 if block[0] == 0xff && block[1] == 0xff {
//                     unk_blocks.push(cur_blocks);
//                     cur_blocks = Vec::<&[u8]>::new();
//                 }
//             }

//             unk_blocks.push(cur_blocks);
//         }

//         offset += chunk_size;
//         chunk_id += 1;
//     }
// }

pub fn dump_sprites(buf: Vec<u8>, texture_path: &Path, output_path: &Path) {
    let val1 = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
    let val2 = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;

    let index_format = if val1 > buf.len() && val2 > buf.len() {
        SourcePlatform::Firebeat
    } else if val1 > buf.len() && val2 <= buf.len() {
        SourcePlatform::PC
    } else {
        SourcePlatform::Python
    };

    let chunk_size = read_formatted_u32_special(&index_format, &buf, 0) as usize;
    let chunk = &buf[4..4+chunk_size];

    let sprites_table_offset = read_formatted_u32(&index_format, chunk, 0x04) as usize;

    let atlas = generate_atlas(&index_format, chunk, texture_path);

    let mut i = 0;
    let mut sprite_idx = 0;

    fs::create_dir_all(output_path).expect("Could not create output directory");

    while sprites_table_offset + i < chunk.len() {
        let x = read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x00) as u32;
        let y = read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x02) as u32;
        let w = read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x04) as u32;
        let h = read_formatted_u16(&index_format, chunk, sprites_table_offset + i + 0x06) as u32;
        i += 8;

        if w == 0 || h == 0 {
            break;
        }

        let chunk = atlas.view(x, y, w, h).to_image();
        let output_filename = output_path.join(format!("{:05}.png", sprite_idx));
        chunk
            .save(&output_filename)
            .expect("Could not save sprite subimage");
        println!("Saved sprite: {:?}", output_filename);
        sprite_idx += 1;
    }

    atlas.save(output_path.join("_atlas.png")).expect("Could not save atlas image");
}
