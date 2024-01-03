use std::{error::Error, io::Write};

use fxyt::render;
use gifed::videogif::{Frame, VideoGif};

fn main() -> Result<(), Box<dyn Error>> {
    let Some(program) = std::env::args().nth(1) else {
        eprintln!("Error: please pass the FXYT program as a command line argument.");
        eprintln!(r#"For example: `fxyt "XY^"`."#);
        eprintln!(r#"To run the empty program and produce a pure black image, run `fxyt ""`."#);
        return Ok(());
    };

    let frames = render(&program)?;

    let mut gif = VideoGif::new(256, 256);

    for frame in frames {
        let mut gif_frame: Frame = frame.image.concat().into();
        gif_frame.set_interval((frame.interval / 10) as u16);
        gif.add_frame(gif_frame);
    }

    let gif = gif.build()?; //do a global palette calculation here if any frames don't have their own palettes?

    let mut output_file = std::fs::File::create("output.gif")?;
    output_file.write_all(&gif.as_bytes())?;

    Ok(())
}
