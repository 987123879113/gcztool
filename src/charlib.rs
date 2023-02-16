use super::gcz::load_texture_from_file;
use image::{GenericImage, GenericImageView, RgbaImage};
use std::{collections::HashMap, fs, path::Path};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum SourcePlatform {
    Firebeat,
    Python,
    PC,
}

struct CharlibBinaryChunk {
    platform: SourcePlatform,
    buf: Vec<u8>,
}

struct CharlibIndex {
    chunks: Vec<CharlibBinaryChunk>,

    sprite_chunk: CharlibSprites,
}

struct CharlibSprites {
    atlas: RgbaImage,
    sprites: Vec<CharlibSprite>,
}

struct CharlibSprite {
    name: Option<String>,
    sprite_id: u16,

    x: u32,
    y: u32,
    w: u32,
    h: u32,

    sprite: RgbaImage,
}

impl CharlibIndex {
    fn new(buf: Vec<u8>, texture_path: &Path) -> CharlibIndex {
        let platform = {
            // Detect the platform based on the format of the data inside the index file
            let val1 = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
            let val2 = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;

            if val1 > buf.len() && val2 > buf.len() {
                SourcePlatform::Firebeat
            } else if val1 > buf.len() && val2 <= buf.len() {
                SourcePlatform::PC
            } else {
                SourcePlatform::Python
            }
        };

        let chunks = {
            let mut chunks = Vec::<CharlibBinaryChunk>::new();
            let mut offset = 0;

            while offset < buf.len() {
                let chunk_size_bytes = buf[offset..offset + 4].try_into().unwrap();

                let chunk_size = match platform {
                    SourcePlatform::Firebeat => u32::from_be_bytes(chunk_size_bytes),
                    SourcePlatform::Python => u32::from_le_bytes(chunk_size_bytes),
                    SourcePlatform::PC => u32::from_be_bytes(chunk_size_bytes),
                } as usize;
                offset += 4;

                chunks.push(CharlibBinaryChunk {
                    platform,
                    buf: buf[offset..offset + chunk_size].to_vec(),
                });
                offset += chunk_size;
            }

            chunks
        };

        let sprite_names = {
            // TODO: This chunk also seems to contain more (animation and maybe layer names?)
            let mut sprite_names = HashMap::<u16, String>::new();

            if chunks.len() > 1 {
                let mut offset = 0;
                while offset + 3 < chunks[1].len() {
                    let mut end = offset;

                    if chunks[1].buf[end] == 0 {
                        break;
                    }

                    while chunks[1].buf[end] != 0 {
                        end += 1;
                    }

                    let name = String::from_utf8(chunks[1].buf[offset..end].try_into().unwrap())
                        .expect("Could not get sprite name as string");
                    offset = end + 1;

                    let k =
                        u16::from_le_bytes(chunks[1].buf[offset..offset + 2].try_into().unwrap());
                    offset += 2;

                    sprite_names.insert(k, name);
                }
            }

            sprite_names
        };

        let atlas = CharlibSprites::generate_atlas(&chunks[0], texture_path);
        let sprite_coords = {
            let mut output = Vec::<CharlibSprite>::new();
            let sprites_table_offset = chunks[0].read_u32(0x04) as usize;
            let mut sprite_idx = 0;

            while sprites_table_offset + (sprite_idx * 8) < chunks[0].len() {
                let x = chunks[0].read_u16(sprites_table_offset + (sprite_idx * 8) + 0x00) as u32;
                let y = chunks[0].read_u16(sprites_table_offset + (sprite_idx * 8) + 0x02) as u32;
                let w = chunks[0].read_u16(sprites_table_offset + (sprite_idx * 8) + 0x04) as u32;
                let h = chunks[0].read_u16(sprites_table_offset + (sprite_idx * 8) + 0x06) as u32;

                if w == 0 || h == 0 {
                    break;
                }

                let sprite_id = sprite_idx as u16;
                output.push(CharlibSprite {
                    sprite_id: sprite_id,
                    name: match sprite_names.get(&sprite_id) {
                        Some(sprite_name) => Some(sprite_name.clone()),
                        None => None,
                    },
                    x,
                    y,
                    w,
                    h,
                    sprite: atlas.view(x, y, w, h).to_image(),
                });

                sprite_idx += 1;
            }

            output
        };

        let sprites = CharlibSprites {
            atlas: atlas,
            sprites: sprite_coords,
        };

        CharlibIndex { chunks, sprite_chunk: sprites }
    }
}

impl CharlibSprites {
    fn generate_atlas(chunk: &CharlibBinaryChunk, texture_path: &Path) -> RgbaImage {
        let image_count = chunk.read_u16(0x02);
        let mut textures = Vec::<RgbaImage>::new();
        let mut texture_w = 0;
        let mut texture_h = 0;

        for i in 0..image_count {
            let start_offset = (0x14 + (i * 0x20)) as usize;
            let end_offset = {
                let mut j = 0;

                for _ in 0..0x20 {
                    if chunk.buf[start_offset + j] == 0 {
                        break;
                    }

                    j += 1;
                }

                start_offset + j
            };

            let filename =
                &String::from_utf8(chunk.buf[start_offset..end_offset].try_into().unwrap())
                    .expect("Could not convert filename");
            let filename = texture_path.join(Path::new(filename).strip_prefix("/").unwrap());
            let tex = load_texture_from_file(filename.as_os_str(), chunk.platform);

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
}

impl CharlibBinaryChunk {
    fn read_u32(&self, offset: usize) -> u32 {
        let bytes = self.buf[offset..offset + 4].try_into().unwrap();

        match self.platform {
            SourcePlatform::Firebeat => u32::from_be_bytes(bytes),
            SourcePlatform::Python => u32::from_le_bytes(bytes),
            SourcePlatform::PC => u32::from_le_bytes(bytes),
        }
    }

    fn read_u16(&self, offset: usize) -> u16 {
        let bytes = self.buf[offset..offset + 2].try_into().unwrap();

        match self.platform {
            SourcePlatform::Firebeat => u16::from_be_bytes(bytes),
            SourcePlatform::Python => u16::from_le_bytes(bytes),
            SourcePlatform::PC => u16::from_le_bytes(bytes),
        }
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}

pub fn dump_sprites(buf: Vec<u8>, texture_path: &Path, output_path: &Path) {
    let parsed_index = CharlibIndex::new(buf, texture_path);

    fs::create_dir_all(output_path).expect("Could not create output directory");

    for sprite in parsed_index.sprite_chunk.sprites {
        let sprite_filename = match sprite.name {
            Some(sprite_name) => format!("{:05}_{}.png", sprite.sprite_id, sprite_name),
            None => format!("{:05}.png", sprite.sprite_id),
        };
        let output_filename = output_path.join(sprite_filename);

        sprite
            .sprite
            .save(&output_filename)
            .expect("Could not save sprite subimage");

        println!("Saved sprite: {:?}", output_filename);
    }

    parsed_index.sprite_chunk
        .atlas
        .save(output_path.join("_atlas.png"))
        .expect("Could not save atlas image");
}
