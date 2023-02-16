use super::gcz::load_texture_from_file;
use image::{GenericImage, GenericImageView, RgbaImage};
use std::{collections::HashMap, fs, path::Path};

#[derive(PartialEq, Debug)]
pub enum SourcePlatform {
    Firebeat,
    Python,
    PC,
}

fn read_formatted_u32(format: &SourcePlatform, buf: &Vec<u8>, offset: usize) -> u32 {
    let bytes = buf[offset..offset + 4].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u32::from_be_bytes(bytes),
        SourcePlatform::Python => u32::from_le_bytes(bytes),
        SourcePlatform::PC => u32::from_le_bytes(bytes),
    }
}

fn read_formatted_u32_chunk_size(format: &SourcePlatform, buf: &Vec<u8>, offset: usize) -> u32 {
    let bytes = buf[offset..offset + 4].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u32::from_be_bytes(bytes),
        SourcePlatform::Python => u32::from_le_bytes(bytes),
        SourcePlatform::PC => u32::from_be_bytes(bytes),
    }
}

fn read_formatted_u16(format: &SourcePlatform, buf: &Vec<u8>, offset: usize) -> u16 {
    let bytes = buf[offset..offset + 2].try_into().unwrap();

    match format {
        SourcePlatform::Firebeat => u16::from_be_bytes(bytes),
        SourcePlatform::Python => u16::from_le_bytes(bytes),
        SourcePlatform::PC => u16::from_le_bytes(bytes),
    }
}

fn generate_atlas(format: &SourcePlatform, chunk: &Vec<u8>, texture_path: &Path) -> RgbaImage {
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
        let tex = load_texture_from_file(filename.as_os_str(), &format);

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

//     let platform = if val1 > buf.len() && val2 > buf.len() {
//         SourcePlatform::Firebeat
//     } else if val1 > buf.len() && val2 <= buf.len() {
//         SourcePlatform::PC
//     } else {
//         SourcePlatform::Python
//     };

//     let mut offset = 0;
//     let mut chunk_id = 0;
//     while offset < buf.len() {
//         let chunk_size = read_formatted_u32_chunk_size(&platform, &buf, offset) as usize;
//         offset += 4;

//         let chunk = &buf[offset..offset + chunk_size];

//         if chunk_id == 0 {
//             let sprites_table_offset = read_formatted_u32(&platform, chunk, 0x04) as usize;
//             let sect2_offset = read_formatted_u32(&platform, chunk, 0x08) as usize;
//             let sect3_offset = read_formatted_u32(&platform, chunk, 0x0c) as usize;
//             let sect4_offset = read_formatted_u32(&platform, chunk, 0x10) as usize;

//             let graphic_id = read_formatted_u16(&platform, chunk, 0x00);

//             let atlas = generate_atlas(&platform, chunk, texture_path);

//             // These are called "frames" in charlib
//             let sprites = {
//                 let mut chunks = Vec::<SubImage<&RgbaImage>>::new();
//                 let mut i = 0;
//                 let mut sprite_idx = 0;

//                 loop {
//                     let x =
//                         read_formatted_u16(&platform, chunk, sprites_table_offset + i + 0x00)
//                             as u32;
//                     let y =
//                         read_formatted_u16(&platform, chunk, sprites_table_offset + i + 0x02)
//                             as u32;
//                     let w =
//                         read_formatted_u16(&platform, chunk, sprites_table_offset + i + 0x04)
//                             as u32;
//                     let h =
//                         read_formatted_u16(&platform, chunk, sprites_table_offset + i + 0x06)
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

fn get_detected_platform_from_index_format(buf: &Vec<u8>) -> SourcePlatform {
    let val1 = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
    let val2 = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;

    if val1 > buf.len() && val2 > buf.len() {
        SourcePlatform::Firebeat
    } else if val1 > buf.len() && val2 <= buf.len() {
        SourcePlatform::PC
    } else {
        SourcePlatform::Python
    }
}

fn get_index_chunks(buf: Vec<u8>) -> Vec<Vec<u8>> {
    let platform = get_detected_platform_from_index_format(&buf);
    let mut chunks = Vec::<Vec<u8>>::new();
    let mut offset = 0;

    while offset < buf.len() {
        let chunk_size = read_formatted_u32_chunk_size(&platform, &buf, offset) as usize;
        offset += 4;

        chunks.push(buf[offset..offset + chunk_size].to_vec());
        offset += chunk_size;
    }

    chunks
}

fn read_names_from_chunk(buf: &Vec<u8>) -> HashMap<u16, String> {
    // TODO: This chunk also seems to contain more (animation and maybe layer names?)
    let mut sprite_names = HashMap::<u16, String>::new();

    let mut offset = 0;
    while offset + 3 < buf.len() {
        let mut end = offset;

        if buf[end] == 0 {
            break;
        }

        while buf[end] != 0 {
            end += 1;
        }

        let name = String::from_utf8(buf[offset..end].try_into().unwrap())
            .expect("Could not get sprite name as string");
        offset = end + 1;

        let k = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        sprite_names.insert(k, name);
    }

    sprite_names
}

pub fn dump_sprites(buf: Vec<u8>, texture_path: &Path, output_path: &Path) {
    let platform = get_detected_platform_from_index_format(&buf);

    let chunks = get_index_chunks(buf);

    if chunks.len() > 2 {
        println!("Found more than 2 chunks! {:?}", chunks.len());
    }

    let sprite_names = read_names_from_chunk(&chunks[1]);

    let sprites_table_offset = read_formatted_u32(&platform, &chunks[0], 0x04) as usize;

    let atlas = generate_atlas(&platform, &chunks[0], texture_path);

    let mut i = 0;
    let mut sprite_idx = 0 as u16;

    fs::create_dir_all(output_path).expect("Could not create output directory");

    while sprites_table_offset + i < chunks[0].len() {
        let x = read_formatted_u16(&platform, &chunks[0], sprites_table_offset + i + 0x00) as u32;
        let y = read_formatted_u16(&platform, &chunks[0], sprites_table_offset + i + 0x02) as u32;
        let w = read_formatted_u16(&platform, &chunks[0], sprites_table_offset + i + 0x04) as u32;
        let h = read_formatted_u16(&platform, &chunks[0], sprites_table_offset + i + 0x06) as u32;
        i += 8;

        if w == 0 || h == 0 {
            break;
        }

        let chunk = atlas.view(x, y, w, h).to_image();
        let sprite_filename = match sprite_names.get(&sprite_idx) {
            Some(sprite_name) => format!("{:05}_{}.png", sprite_idx, sprite_name),
            None => format!("{:05}.png", sprite_idx),
        };
        let output_filename = output_path.join(sprite_filename);
        chunk
            .save(&output_filename)
            .expect("Could not save sprite subimage");
        println!("Saved sprite: {:?}", output_filename);
        sprite_idx += 1;
    }

    atlas
        .save(output_path.join("_atlas.png"))
        .expect("Could not save atlas image");
}
