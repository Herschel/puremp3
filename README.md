# puremp3
[![crates.io](https://img.shields.io/crates/v/puremp3.svg)](https://crates.io/crates/puremp3)
[![docs.rs](https://docs.rs/puremp3/badge.svg)](https://docs.rs/puremp3)
[![CircleCI](https://circleci.com/gh/Herschel/puremp3.svg?style=svg)](https://circleci.com/gh/Herschel/puremp3)

An MP3 decoder written in pure Rust.

The motivation for this crate is to create a pure Rust MP3 decoder that easily compiles to the `wasm32-unknown-unknown` target. No claims are made to accuracy, performance, or compatibility. For a more robust decoder, try [minimp3-rs](https://github.com/germangb/minimp3-rs).

## Support

* MPEG-1/MPEG-2/MPEG-2.5 Layer III

## Example

```
let data = std::fs::read("tests/vectors/MonoCBR192.mp3").expect("Could not open file");
let (header, samples) = puremp3::read_mp3(&data[..]).expect("Invalid MP3");
for (left, right) in samples {
    // Operate on samples here
}
```

## Prior art

The following implementations and documents were referenced in creating this crate:

 * [PDMP3](https://github.com/technosaurus/PDMP3)
 * [OpenMP3](https://github.com/audioboy77/OpenMP3)
 * [minimp3](https://github.com/lieff/minimp3)
 * [minimp3-rs](https://github.com/germangb/minimp3-rs)
 * [Let's build an MP3-decoder!](http://blog.bjrn.se/2008/10/lets-build-mp3-decoder.html)
 * [MP3 Decoder Master's Thesis](https://sites.google.com/a/kmlager.com/www/projects)
 
## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 * Creative Commons CC0 1.0 Universal Public Domain Dedication ([LICENSE-CC0](LICENSE-CC0) or https://creativecommons.org/publicdomain/zero/1.0/)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.
