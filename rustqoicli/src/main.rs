use clap::{Arg, ArgAction, Command};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use rustqoi::*;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

fn prompt_overwrite(file: &str) -> bool {
    print!("The file '{}' already exists. Overwrite? [y/N]: ", file);
    io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
    let mut response = String::new();
    io::stdin().read_line(&mut response).expect("Failed to read input");
    matches!(response.trim().to_lowercase().as_str(), "y" | "yes")
}

fn derive_output_filename(input: &str) -> String {
    let path = Path::new(input);
    let new_extension = match path.extension().and_then(|ext| ext.to_str()) {
        Some("png") => "qoi",
        Some("qoi") => "png",
        _ => panic!("Unsupported file extension. Use `.png` or `.qoi`.")
    };
    path.with_extension(new_extension).to_string_lossy().into_owned()
}

fn check_and_prepare_output(output: &str, force: bool) -> bool {
    force || !Path::new(output).exists() || prompt_overwrite(output)
}

fn decode_png_to_vecu8(image: DynamicImage) -> (u32, u32, Channels, Vec<u8>) {
    let (width, height) = image.dimensions();
    let (channels, pixels) = match image.color().channel_count() {
        3 => (Channels::RGB, image.to_rgb8().into_raw()),
        4 => (Channels::RGBA, image.to_rgba8().into_raw()),
        _ => panic!("Unsupported number of channels"),
    };
    (width, height, channels, pixels)
}

fn encode(input: &str, output: &str, verbose: usize, force: bool) {
    let image = match image::open(input) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Error: Failed to open input image: {}", e);
            return;
        }
    };
    let (width, height, channels, rgb_values) = decode_png_to_vecu8(image);
    let desc = QoiHeader { width, height, channels, colorspace: Colorspace::Linear };

    if verbose > 0 {
        println!("Encoding {} -> {}", input, output);
        println!("Dimensions: {}x{}, Channels: {:?}", width, height, channels);
    }

    if !check_and_prepare_output(output, force) {
        println!("Operation canceled.");
        return;
    }

    let start = Instant::now();
    let qoi_encoded_bytes = qoi_encode(&rgb_values, &desc);
    let duration = start.elapsed();

    if let Err(e) = fs::write(output, qoi_encoded_bytes) {
        eprintln!("Error: Failed to write QOI data: {}", e);
        return;
    }

    if verbose > 0 {
        println!("Encoding completed in {:.2?}.", duration);
    }
}

fn decode(input: &str, output: &str, verbose: usize, force: bool) {
    let qoi_buffer = match fs::read(input) {
        Ok(buf) => buf,
        Err(e) => {
            eprintln!("Error: Failed to read QOI file: {}", e);
            return;
        }
    };
    let start = Instant::now();
    let (decoded_buffer, qoi_desc) = qoi_decode(&qoi_buffer);
    let duration = start.elapsed();

    if verbose > 0 {
        println!("Decoding {} -> {}", input, output);
        println!("  Dimensions: {}x{}", qoi_desc.width, qoi_desc.height);
    }

    if !check_and_prepare_output(output, force) {
        println!("Operation canceled.");
        return;
    }

    let image_buffer = match ImageBuffer::<Rgba<u8>, _>::from_raw(
        qoi_desc.width, qoi_desc.height, decoded_buffer
    ) {
        Some(buf) => buf,
        None => {
            eprintln!("Error: Failed to create image buffer");
            return;
        }
    };

    if let Err(e) = image_buffer.save(output) {
        eprintln!("Error: Failed to save PNG image: {}", e);
        return;
    }

    if verbose > 0 {
        println!("Decoding completed in {:.2?}.", duration);
    }
}

fn main() {
    let matches = Command::new("rustqoi")
        .version("0.1.0")
        .author("musicalskele")
        .about("simple QOI encoder/decoder")
        .arg(Arg::new("input").required(true).help("input file (.png or .qoi)"))
        .arg(Arg::new("output").help("optional output file"))
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count).help("toggle verbosity (-v)"))
        .arg(Arg::new("force").short('y').long("yes").action(ArgAction::SetTrue).help("force overwrite output file without asking"))
        .get_matches();

    let input = matches.get_one::<String>("input").unwrap();
    let output = matches.get_one::<String>("output").cloned().unwrap_or_else(|| derive_output_filename(input));
    let verbose = matches.get_count("verbose") as usize;
    let force = matches.get_flag("force");

    match Path::new(input).extension().and_then(|ext| ext.to_str()) {
        Some("png") => encode(input, &output, verbose, force),
        Some("qoi") => decode(input, &output, verbose, force),
        _ => eprintln!("Unsupported file format. Use `.png` or `.qoi`!"),
    }
}
