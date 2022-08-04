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
use std::path::PathBuf;

// re-create this type so we can derive ValueEnum on it
/// Image resampling filter types
#[derive(ValueEnum, Clone, Copy)]
enum FilterType {
    /// Nearest-neighbour resampling
    Nearest,

    /// Linear (triangle) resampling
    Triangle,

    /// Cubic (Catmull-Rom) resampling
    Cubic,

    /// Gaussian resampling
    Gaussian,

    /// Lanczos resampling with window 3
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
    /// Which resampling filter to use when resizing the image
    filter: FilterType,

    /// If enabled, any warnings will stop all processing
    #[clap(long)]
    stop_on_warning: bool,
}

fn main() -> Result<()> {
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

    let im = ImageReader::open(&image)
        .with_context(|| format!("Failed to open file '{}'", image.display()))?
        .decode()
        .with_context(|| "Failed to decode image!")?;

    if im.width() != im.height() {
        eprintln!(
            "{}: your input image is not square, and will appear squished!",
            console::style("Warning").yellow()
        );
        if stop_on_warning {
            return Err(anyhow!("Input image isn't square!"));
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
        .iter()
        .map(|&sz| {
            let im = im.resize_exact(sz, sz, filter.into());
            im.to_rgba8().to_vec()
        })
        .collect();

    let frames: Result<Vec<IcoFrame>> = frames
        .iter()
        .zip(sizes.iter())
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
