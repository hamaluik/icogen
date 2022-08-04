// Copyright 2022 Kenton Hamaluik
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   //http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::io::Reader as ImageReader;
use image::{DynamicImage, Rgba, RgbaImage};
use rayon::prelude::*;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ExitCode;

// re-create this type so we can derive ValueEnum on it
/// Image re-sampling filter types
#[derive(ValueEnum, Clone, Copy)]
enum FilterType {
    /// Nearest-neighbour re-sampling
    Nearest,

    /// Linear (triangle) re-sampling
    Triangle,

    /// Cubic (Catmull-Rom) re-sampling
    Cubic,

    /// Gaussian re-sampling
    Gaussian,

    /// Lanczos re-sampling with window 3
    Lanczos,
}

impl Default for FilterType {
    fn default() -> FilterType {
        FilterType::Cubic
    }
}

impl From<FilterType> for image::imageops::FilterType {
    fn from(t: FilterType) -> Self {
        match t {
            FilterType::Nearest => image::imageops::FilterType::Nearest,
            FilterType::Triangle => image::imageops::FilterType::Triangle,
            FilterType::Cubic => image::imageops::FilterType::CatmullRom,
            FilterType::Gaussian => image::imageops::FilterType::Gaussian,
            FilterType::Lanczos => image::imageops::FilterType::Lanczos3,
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The image file to convert
    image: PathBuf,

    #[clap(short, long, default_values_t = vec![16, 20, 24, 32, 40, 48, 64, 96, 128, 256])]
    /// What sizes of icon to generate
    sizes: Vec<u32>,

    #[clap(short, long, value_enum, default_value_t = FilterType::default())]
    /// Which re-sampling filter to use when resizing the image
    filter: FilterType,

    /// If enabled, any warnings will stop all processing
    #[clap(long)]
    stop_on_warning: bool,
}

fn main() -> ExitCode {
    if let Err(e) = try_main() {
        eprintln!("{}: {e:#}", console::style("Error").red());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn try_main() -> Result<()> {
    let Cli {
        image,
        mut sizes,
        filter,
        stop_on_warning,
    } = Cli::parse();

    sizes.sort();

    if !image.is_file() {
        return Err(anyhow!("Path '{}' isn't a file!", image.display()));
    }

    let output = image.file_stem().unwrap().to_string_lossy().to_string();
    let output = PathBuf::from(format!("{output}.ico"));

    if output.exists() {
        eprintln!(
            "{}: the file '{}' already exists!",
            console::style("Warning").yellow(),
            output.display()
        );
        if stop_on_warning {
            return Err(anyhow!("Program would overwrite existing icon"));
        }
    }

    let mut removed_sizes: Vec<u32> = Vec::default();
    let sizes: Vec<u32> = sizes
        .into_iter()
        .filter(|&s| {
            let keep = s >= 1 && s <= 256;
            if !keep {
                removed_sizes.push(s);
            }
            keep
        })
        .collect();

    if !removed_sizes.is_empty() {
        eprintln!(
            "{}: The following sizes were removed because they are too big (or too small): {}",
            console::style("Warning").yellow(),
            removed_sizes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        );
        if stop_on_warning {
            return Err(anyhow!("Input image would be scaled up!"));
        }
    }

    if sizes.is_empty() {
        eprintln!(
            "{}: No sizes were marked for the icon, aborting!",
            console::style("Error").red(),
        );
        return Ok(());
    }

    let im: DynamicImage = if image
        .extension()
        .map(OsStr::to_str)
        .flatten()
        .map(str::to_lowercase)
        == Some("svg".to_owned())
    {
        let mut opt = usvg::Options::default();
        opt.resources_dir = std::fs::canonicalize(&image)
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));
        opt.fontdb.load_system_fonts();

        let svg = std::fs::read(&image)
            .with_context(|| format!("Failed to read file '{}'", image.display()))?;
        let rtree = usvg::Tree::from_data(&svg, &opt.to_ref())
            .with_context(|| "Failed to parse SVG contents")?;

        let pixmap_size = rtree.svg_node().size.to_screen_size();

        if pixmap_size.width() != pixmap_size.height() {
            eprintln!(
                "{}: your input image is not square, and will appear squished!",
                console::style("Warning").yellow()
            );
            if stop_on_warning {
                return Err(anyhow!("Input image isn't square!"));
            }
        }

        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
            .with_context(|| "Failed to create SVG Pixmap!")?;

        let size = *sizes.iter().max().unwrap();
        resvg::render(
            &rtree,
            usvg::FitTo::Size(size, size),
            tiny_skia::Transform::default(),
            pixmap.as_mut(),
        )
        .with_context(|| "Failed to render SVG!")?;

        // copy it into an image buffer translating types as we go
        // I'm sure there's faster ways of doing this but ¯\_(ツ)_/¯
        let mut image = RgbaImage::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let pixel = pixmap.pixel(x, y).unwrap();
                let pixel = Rgba([pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]);
                image.put_pixel(x, y, pixel);
            }
        }

        image.into()
    } else {
        ImageReader::open(&image)
            .with_context(|| format!("Failed to open file '{}'", image.display()))?
            .decode()
            .with_context(|| "Failed to decode image!")?
    };

    if im.width() != im.height() {
        eprintln!(
            "{}: your input image is not square, and will appear squished!",
            console::style("Warning").yellow()
        );
        if stop_on_warning {
            return Err(anyhow!("Input image isn't square!"));
        }
    }

    if im.width() < sizes.iter().max().map(|&v| v).unwrap_or_default() {
        eprintln!(
            "{}: You've requested sizes bigger than your input, your image will be scaled up!",
            console::style("Warning").yellow()
        );
        if stop_on_warning {
            return Err(anyhow!("Input image would be scaled up!"));
        }
    }

    println!(
        "Converting {} to {} with sizes [{}]...",
        image.display(),
        output.display(),
        sizes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    );

    let frames: Vec<Vec<u8>> = sizes
        .par_iter()
        .map(|&sz| {
            let im = im.resize_exact(sz, sz, filter.into());
            im.to_rgba8().to_vec()
        })
        .collect();

    let frames: Result<Vec<IcoFrame>> = frames
        .par_iter()
        .zip(sizes.par_iter())
        .map(|(buf, &sz)| {
            IcoFrame::as_png(buf.as_slice(), sz, sz, im.color())
                .with_context(|| "Failed to encode frame")
        })
        .collect();
    let frames = frames?;

    let file = std::fs::File::create(&output)
        .with_context(|| format!("Failed to create file '{}'", output.display()))?;
    let encoder = IcoEncoder::new(file);
    encoder
        .encode_images(frames.as_slice())
        .with_context(|| "Failed to encode .ico file")?;

    println!("Icon saved to '{}'!", output.display());
    Ok(())
}
