pub mod charlib;
pub mod gcz;

use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use charlib::dump_sprites;
use gcz::{gcz_decompress, load_texture_from_file};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Name of the person to greet
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Convert a GCZ texture file to an image file
    #[command(arg_required_else_help = true)]
    GczDump {
        // Compresed GCZ file
        #[arg(short, long, required = true)]
        input_filename: PathBuf,

        // Path to save image file
        #[arg(short, long, required = true)]
        output_filename: PathBuf,

        // Platform where the data originiated
        #[arg(short, long, required = true, value_enum)]
        platform: SourcePlatform,
    },

    /// Decompress a GCZ file
    #[command(arg_required_else_help = true)]
    GczDecomp {
        // Compresed GCZ file
        #[arg(short, long, required = true)]
        input_filename: PathBuf,

        // Path to save raw decompressed GCZ file
        #[arg(short, long, required = true)]
        output_filename: PathBuf,
    },

    /// Split a set of GCZ files into individual sprites using associated index file
    #[command(arg_required_else_help = true)]
    IdxDumpSprites {
        // Index file containing GCZ references, sprite information, animations, etc
        #[arg(short, long, required = true)]
        input_filename: PathBuf,

        // Path relative to where the game expects to be able to find the referenced GCZ files
        #[arg(short, long, required = true)]
        graphics_path: PathBuf,

        // Path to dump all of the individual sprite images
        #[arg(short, long, required = true)]
        output_path: PathBuf,
    },
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum SourcePlatform {
    PC,
    Python,
    Firebeat,
}

impl std::fmt::Display for SourcePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

fn get_format_from_platform(platform: SourcePlatform) -> charlib::SourcePlatform {
    match platform {
        SourcePlatform::PC => charlib::SourcePlatform::PC,
        SourcePlatform::Firebeat => charlib::SourcePlatform::Firebeat,
        SourcePlatform::Python => charlib::SourcePlatform::Python,
    }
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::GczDump {
            input_filename,
            output_filename,
            platform,
        } => {
            let format = get_format_from_platform(platform);

            let texture = load_texture_from_file(input_filename.as_os_str(), &format);
            texture
                .save(&output_filename)
                .expect("Could not export texture");
            println!("Dumped GCZ data to {:?}", output_filename);
        }
        Commands::GczDecomp {
            input_filename,
            output_filename,
        } => {
            let gcz_data = fs::read(input_filename).expect("Could not read input GCZ file");
            let decomp = gcz_decompress(&gcz_data);
            fs::write(&output_filename, &decomp).expect("Could not write file");
            println!("Dumped raw GCZ data to {:?}", output_filename);
        }
        Commands::IdxDumpSprites {
            input_filename,
            graphics_path,
            output_path,
        } => {
            let buf = fs::read(input_filename).expect("Could not open input index file");
            dump_sprites(buf, &graphics_path, &output_path)
        }
    }
}
