# icogen
Quickly convert image files into Windows `.ico` files

[![Crates.io](https://img.shields.io/crates/v/icogen.svg)](https://crates.io/crates/icogen) ![license](https://img.shields.io/crates/l/icogen) ![maintenance](https://img.shields.io/badge/maintenance-passively--maintained-yellowgreen.svg)

---

I often need to convert an image into a `.ico` file and often turn to some web-based service to do this (just drag and drop the image, out comes a `.ico`, etc). I shouldn't have to go online to do this quickly and easily, hence this tool. It is small and only does 1 thing, and will only ever do one thing, by design. This is a thin CLI wrapper around the [image](https://crates.io/crates/image) crate.

## Usage

```
icogen 1.0.0
Kenton Hamaluik <kenton@hamaluik.ca>
Quickly convert image files into Windows .ico files

USAGE:
    icogen.exe [OPTIONS] <IMAGE>

ARGS:
    <IMAGE>    The image file to convert

OPTIONS:
    -f, --filter <FILTER>    Which resampling filter to use when resizing the image [default: cubic] [possible values: nearest, triangle, cubic, gaussian, lanczos]
    -h, --help               Print help information
    -s, --sizes <SIZES>      What sizes of icon to generate [default: 16 20 24 32 40 48 64 96 128 256]
        --stop-on-warning    If enabled, any warnings will stop all processing
    -V, --version            Print version information
```

## Supported File Formats

Basically what [image](https://crates.io/crates/image) supports for decoding:

* PNG
* JPEG
* GIF
* BMP
* ICO
* TIFF (baseline (no fax support) + LZW + PackBits)
* WebP
* AVIF (only 8-bit)
* PNM (PBM, PGM, PPM, standard PAM)
* DDS (DXT1, DXT3, DXT5)
* TGA
* OpenEXR (Rgb32F, Rgba32F (no dwa compression))
* farbfeld

## Installing

From [crates.io](https://crates.io/) (assuming you have [Rust](https://www.rust-lang.org/) installed): 

```bash
$ cargo install icogen
```

Otherwise, some pre-compied binaries should be available on GitHub: https://github.com/hamaluik/icogen/releases/

## Future Work

* SVG support

