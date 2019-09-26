extern crate clap;
extern crate lodepng;
extern crate exoquant;

use clap::{Arg, App};
use exoquant::*;
use exoquant::optimizer::Optimizer;
use lodepng::Bitmap;
use lodepng::RGBA;
use std::io::Write;
use std::io::Read;
use std::process::exit;

fn main() {
    let matches = App::new("pngeq")
        .version("0.1.0")
        .author("Dennis Ranke <dennis.ranke@gmail.com>")
        .about("Quantize 24/32bit PNG files to 8bit.")
        .arg(Arg::with_name("optimization level")
            .long("opt")
            .short("O")
            .takes_value(true)
            .possible_values(&["0", "s1", "s2", "s3", "c1", "c2", "c3"])
            .help("Palette optimization"))
        .arg(Arg::with_name("ditherer")
            .long("dither")
            .short("d")
            .takes_value(true)
            .possible_values(&["none", "ordered", "fs", "fs-checkered"])
            .help("Ditherer to use"))
        .args_from_usage("<NUM_COLORS> 'target color count for output'
                          <INPUT> 'input truecolor png'
                          <OUTPUT> 'output 8bit png'
                          ")
        .after_help("K-Means optimization levels: none ('0'), optimize for smoothness ('s1' - \
                     's3'), optimize for colors ('c1' - 'c3'). Defaults depend on NUM_COLORS: > \
                     128 color: 's1', > 64 colors: 's2', >= 32 colors: 'c2', < 32 colors: 'c3'\n\
                     Available ditherers: 'none', 'ordered', 'fs', 'fs-checkered'")
        .get_matches();

    let ditherer: Box<ditherer::Ditherer> = match matches.value_of("ditherer")
        .unwrap_or("fs-checkered") {
        "none" => Box::new(ditherer::None),
        "ordered" => Box::new(ditherer::Ordered),
        "fs" => Box::new(ditherer::FloydSteinberg::new()),
        "fs-checkered" => Box::new(ditherer::FloydSteinberg::checkered()),
        other => panic!("Unknown ditherer '{}'", other),
    };
    let num_colors: usize = match matches.value_of("NUM_COLORS").unwrap().parse() {
        Ok(num) if num > 0 && num <= 256 => num,
        _ => {
            writeln!(&mut std::io::stderr(),
                     "Error: NUM_COLORS needs to be an integer between 1-256.")
                .unwrap();
            exit(1)
        }
    };
    let (optimizer, opt_level): (Box<Optimizer>, u32) = match matches.value_of("optimization level")
        .unwrap_or(if num_colors > 128 {
            "s1"
        } else if num_colors > 64 {
            "s2"
        } else if num_colors >= 32 {
            "c2"
        } else {
            "c3"
        }) {
        "0" => (Box::new(optimizer::None), 0),
        "s1" => (Box::new(optimizer::KMeans), 1),
        "s2" => (Box::new(optimizer::KMeans), 2),
        "s3" => (Box::new(optimizer::KMeans), 3),
        "c1" => (Box::new(optimizer::WeightedKMeans), 1),
        "c2" => (Box::new(optimizer::WeightedKMeans), 2),
        "c3" => (Box::new(optimizer::WeightedKMeans), 3),
        other => panic!("Unknown optimization level '{}'", other),
    };

    let input_name = matches.value_of("INPUT").unwrap();
    let img = load_img(input_name);

    let histogram = img.buffer.as_ref().iter().map(|c| Color::new(c.r, c.g, c.b, c.a)).collect();

    let colorspace = SimpleColorSpace::default();

    let mut quantizer = Quantizer::new(&histogram, &colorspace);
    let kmeans_step = if opt_level < 2 {
        num_colors
    } else if opt_level == 2 {
        (num_colors as f64).sqrt().round() as usize
    } else {
        1
    };
    while quantizer.num_colors() < num_colors {
        quantizer.step();
        if quantizer.num_colors() % kmeans_step == 0 {
            quantizer = quantizer.optimize(&*optimizer, 4);
        }
    }

    let palette = quantizer.colors(&colorspace);
    let palette = optimizer.optimize_palette(&colorspace, &palette, &histogram, 8);

    let mut state = lodepng::State::new();
    for color in &palette {
        unsafe {
            lodepng::ffi::lodepng_palette_add(&mut state.info_png().color,
                                              color.r,
                                              color.g,
                                              color.b,
                                              color.a);
            lodepng::ffi::lodepng_palette_add(&mut state.info_raw(),
                                              color.r,
                                              color.g,
                                              color.b,
                                              color.a);
        }
    }
    state.info_png().color.bitdepth = 8;
    state.info_png().color.colortype = lodepng::ColorType::LCT_PALETTE;
    state.info_raw().bitdepth = 8;
    state.info_raw().colortype = lodepng::ColorType::LCT_PALETTE;

    let remapper = Remapper::new(&palette, &colorspace, &*ditherer);
    let out_data: Vec<_> = remapper.remap_iter(Box::new(img.buffer
                        .as_ref()
                        .iter()
                        .map(|c| Color::new(c.r, c.g, c.b, c.a))),
                    img.width)
        .collect();

    let output_name = matches.value_of("OUTPUT").unwrap();
    
    if (output_name == "-") {
        let mut encoded_file = state.encode(&out_data, img.width, img.height).unwrap();
        let mut out = encoded_file.as_mut();
        std::io::stdout().write_all(&out);
    } else {
        match state.encode_file(output_name, &out_data, img.width, img.height) {
            Ok(_) => (),
            Err(_) => {
                writeln!(&mut std::io::stderr(),
                         "Error: Failed to write PNG '{}'.",
                         output_name)
                    .unwrap();
                exit(1)
            }
        };   
    }
}

fn load_img(input_name: &str) -> Bitmap<RGBA<u8>> {
    if (input_name == "-") {
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer);
        let slice = &buffer[..];
        let img = match lodepng::decode32(slice) {
            Ok(img) => img,
            Err(_) => {
                writeln!(&mut std::io::stderr(),
                         "Error: Failed to load PNG '{}'.",
                         input_name)
                    .unwrap();
                exit(1)
            }
        };

        return img;
    }
    
    let img = match lodepng::decode32_file(input_name) {
        Ok(img) => img,
        Err(_) => {
            writeln!(&mut std::io::stderr(),
                     "Error: Failed to load PNG '{}'.",
                     input_name)
                .unwrap();
            exit(1)
        }
    };
    
    return img;
}