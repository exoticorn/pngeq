# pngeq

`pngeq` is a simple command line image quantization tool to convert 24/32 bit
PNG images to 8 bit PNGs based on the
[exoquant](https://github.com/exoticorn/exoquant-rs) library.

# Installation

Install [rust](https://www.rust-lang.org) (including `cargo`), then:

```
cargo install pngeq
```

# Usage

```
USAGE:
    pngeq [OPTIONS] <NUM_COLORS> <INPUT> <OUTPUT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --dither <ditherer>           Ditherer to use
                                      [values: none, ordered, fs, fs-checkered]
    -O, --opt <optimization level>    Palette optimization
                                      [values: 0, s1, s2, s3, c1, c2, c3]

ARGS:
    <NUM_COLORS>    target color count for output
    <INPUT>         path to input truecolor png (use \"-\" to read file from stdin)
    <OUTPUT>        path for output 8bit png (use \"-\" to output to stdout)

K-Means optimization levels: none ('0'), optimize for smoothness ('s1' - 's3'),
optimize for colors ('c1' - 'c3'). Defaults depend on NUM_COLORS: > 128 color:
's1', > 64 colors: 's2', >= 32 colors: 'c2', < 32 colors: 'c3'
Available ditherers: 'none', 'ordered', 'fs', 'fs-checkered'
```
