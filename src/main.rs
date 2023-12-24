use std::{error::Error, io::Write};

use fxyt::{render, ParseError};

fn main() -> Result<(), Box<dyn Error>> {
    let Some(program) = std::env::args().nth(1) else {
        eprintln!("Error: please pass the FXYT program as a command line argument.");
        eprintln!(r#"For example: `fxyt "XY^"`."#);
        eprintln!(r#"To run the empty program and produce a pure black image, run `fxyt ""`."#);
        return Ok(());
    };

    let frames = match render(&program) {
        Ok(frames) => frames,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(());
        }
    };

    let mut gif = gifed::GifBuilder::default();
    gif.set_resolution(256, 256);
    //optional:
    //gif.set_framerate(30);
    //gif.set_palette(gifed::Palette::Simple);

    for frame in &frames {
        //option 1:
        let gif_frame = GifFrameBuilder::from(frame.image);
        gif_frame.set_interval(frame.interval); //if no interval is set on a frame, the gif builder will assign it one
                                                //based on the framerate set above?
        gif_frame.optimize_palette(); //or gif_frame.use_global_palette();
        gif.add_frame(gif_frame.build()?);
        //option 2:
        gif.add_frame(frame.image, frame.interval).unwrap(); //could throw errors when given an image of the wrong resolution?
    }

    let gif = gif.build()?; //do a global palette calculation here if any frames don't have their own palettes?

    let output_file = std::fs::File::create("output.gif")?;
    output_file.write_all(gif.into_bytes())?;

    Ok(())
}
