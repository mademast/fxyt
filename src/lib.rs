use std::fmt::Display;

use rgb::RGB8;
use thiserror::Error;

pub fn render(program: &str) -> Result<Vec<[[RGB8; 256]; 256]>, FxytError> {
    let parsed = parse(program, 0, 0)?.1;

    let t_range = if program.contains(|c| c == 'T' || c == 't') {
        0..256
    } else {
        0..1
    };

    let mut frames = Vec::with_capacity(t_range.len());
    for t in t_range {
        let mut canvas = [[RGB8::default(); 256]; 256];

        for x in 0..256 {
            #[allow(clippy::needless_range_loop)] //this is cleaner than what clippy wants
            for y in 0..256 {
                canvas[255 - y][x] = render_to_pixel(&parsed, Coords::new(x, y, t))?;
            }
        }
        frames.push(canvas);
    }

    Ok(frames)
}

fn render_to_pixel(commands: &[Command], coords: Coords) -> Result<RGB8, FxytError> {
    let mut stack = Vec::with_capacity(8);
    let mut mode = 0;

    if let Some(colour) = render_to_stack(commands, &mut stack, &mut mode, coords)? {
        return Ok(colour);
    }

    let blue = stack.pop().unwrap_or_default();
    let green = stack.pop().unwrap_or_default();
    let red = stack.pop().unwrap_or_default();

    if red > 255 || green > 255 || blue > 255 || red < 0 || green < 0 || blue < 0 {
        return Err(FxytError::RgbOutOfRange);
    }

    Ok(RGB8::new(red as u8, green as u8, blue as u8))
}

fn render_to_stack(
    commands: &[Command],
    stack: &mut Vec<isize>,
    mode: &mut u8,
    coords: Coords,
) -> Result<Option<RGB8>, FxytError> {
    for command in commands {
        match command {
            Command::Coordinates(c) => match c {
                Coordinates::X => stack.push(coords.x),
                Coordinates::Y => stack.push(coords.y),
                Coordinates::T => stack.push(coords.t),
            },
            Command::Integer => stack.push(0),
            Command::Digit(d) => {
                let top = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push(top * 10 + *d as isize)
            }
            Command::Arithmetic(a) => {
                let right = stack.pop().ok_or(FxytError::StackEmpty)?;
                let left = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push(match a {
                    Arithmetic::Plus => left + right,
                    Arithmetic::Minus => left - right,
                    Arithmetic::Times => left * right,
                    Arithmetic::Divide => {
                        if right != 0 {
                            left / right
                        } else {
                            match mode {
                                0 => return Err(FxytError::DivideByZero),
                                1 => return Ok(Some(RGB8::default())),
                                2 => return Ok(Some(RGB8::new(255, 0, 0))),
                                _ => unreachable!(),
                            }
                        }
                    }
                    Arithmetic::Modulus => left % right,
                })
            }
            Command::Mode => *mode += 1,
            Command::Comparison(c) => {
                let right = stack.pop().ok_or(FxytError::StackEmpty)?;
                let left = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push(match c {
                    Comparison::Equals => left == right,
                    Comparison::LessThan => left < right,
                    Comparison::GreaterThan => left > right,
                } as isize)
            }
            Command::Invert => {
                let arg = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push((arg == 0) as isize)
            }
            Command::Bitwise(b) => {
                let right = stack.pop().ok_or(FxytError::StackEmpty)?;
                let left = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push(match b {
                    Bitwise::Xor => left ^ right,
                    Bitwise::And => left & right,
                    Bitwise::Or => left | right,
                })
            }
            Command::Clip => {
                let arg = stack.pop().ok_or(FxytError::StackEmpty)?;
                stack.push(arg.clamp(0, 255))
            }
            Command::StackOperation(so) => match so {
                StackOperation::Duplicate => {
                    let arg = stack.pop().ok_or(FxytError::StackEmpty)?;
                    stack.push(arg);
                    stack.push(arg);
                }
                StackOperation::Pop => {
                    stack.pop().ok_or(FxytError::StackEmpty)?;
                }
                StackOperation::Swap => {
                    let right = stack.pop().ok_or(FxytError::StackEmpty)?;
                    let left = stack.pop().ok_or(FxytError::StackEmpty)?;
                    stack.push(right);
                    stack.push(left);
                }
                StackOperation::Rotate => {
                    let top = stack.pop().ok_or(FxytError::StackEmpty)?;
                    let second = stack.pop().ok_or(FxytError::StackEmpty)?;
                    let third = stack.pop().ok_or(FxytError::StackEmpty)?;
                    stack.extend_from_slice(&[second, top, third])
                }
            },
            Command::Loop(inner_commands) => {
                if let Some(colour) = render_to_stack(inner_commands, stack, mode, coords)? {
                    return Ok(Some(colour));
                }
            }
            Command::FrameInterval => unimplemented!(),
            Command::Debug => {
                eprintln!("{coords} -> {:?}", stack);
                return Err(FxytError::DebugHalt);
            }
        }
        if stack.len() > 8 {
            return Err(FxytError::StackOverflow);
        }
        if *mode > 2 {
            return Err(FxytError::ModeOutOfRange);
        }
    }

    Ok(None)
}

fn parse(program: &str, offset: usize, nesting: u8) -> Result<(usize, Vec<Command>), ParseError> {
    let mut parsed = Vec::with_capacity(program.len());
    let mut unparsed = program.chars().skip(offset);

    let mut index = offset;
    while let Some(c) = unparsed.next() {
        if !c.is_ascii() {
            return Err(ParseError::InvalidCharacter(index));
        }

        let next_command = match c.to_ascii_uppercase() {
            'X' | 'Y' | 'T' => Command::Coordinates(match c {
                'X' => Coordinates::X,
                'Y' => Coordinates::Y,
                'T' => Coordinates::T,
                _ => unreachable!(),
            }),
            'N' => Command::Integer,
            d if d.is_ascii_digit() => Command::Digit(d.to_digit(10).unwrap() as u8),
            '+' | '-' | '*' | '/' | '%' => Command::Arithmetic(match c {
                '+' => Arithmetic::Plus,
                '-' => Arithmetic::Minus,
                '*' => Arithmetic::Times,
                '/' => Arithmetic::Divide,
                '%' => Arithmetic::Modulus,
                _ => unreachable!(),
            }),
            'M' => Command::Mode,
            '=' | '<' | '>' => Command::Comparison(match c {
                '=' => Comparison::Equals,
                '<' => Comparison::LessThan,
                '>' => Comparison::GreaterThan,
                _ => unreachable!(),
            }),
            '!' => Command::Invert,
            '^' | '&' | '|' => Command::Bitwise(match c {
                '^' => Bitwise::Xor,
                '&' => Bitwise::And,
                '|' => Bitwise::Or,
                _ => unreachable!(),
            }),
            'C' => Command::Clip,
            'D' | 'P' | 'S' | 'R' => Command::StackOperation(match c {
                'D' => StackOperation::Duplicate,
                'P' => StackOperation::Pop,
                'S' => StackOperation::Swap,
                'R' => StackOperation::Rotate,
                _ => unreachable!(),
            }),
            '[' => {
                if nesting >= 8 {
                    return Err(ParseError::LoopNesting);
                } else {
                    let (eaten, loop_body) = parse(program, index + 1, nesting + 1)?;
                    index += eaten;
                    unparsed.nth(eaten);

                    Command::Loop(loop_body)
                }
            }
            ']' => {
                if nesting > 0 {
                    return Ok((index - offset + 1, parsed));
                } else {
                    return Err(ParseError::InvalidCharacter(index));
                }
            }
            'F' => Command::FrameInterval,
            'W' => Command::Debug,

            _ => return Err(ParseError::InvalidCharacter(index)),
        };

        index += 1;

        parsed.push(next_command);
    }

    Ok((index - offset, parsed))
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Command {
    Coordinates(Coordinates),
    Integer,
    Digit(u8),
    Arithmetic(Arithmetic),
    Mode,
    Comparison(Comparison),
    Invert,
    Bitwise(Bitwise),
    Clip,
    StackOperation(StackOperation),
    Loop(Vec<Command>),
    FrameInterval,
    Debug,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Coordinates {
    X,
    Y,
    T,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arithmetic {
    Plus,
    Minus,
    Times,
    Divide,
    Modulus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Comparison {
    Equals,
    LessThan,
    GreaterThan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Bitwise {
    Xor,
    And,
    Or,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StackOperation {
    Duplicate,
    Pop,
    Swap,
    Rotate,
}

#[derive(Clone, Copy)]
struct Coords {
    x: isize,
    y: isize,
    t: isize,
}

impl Coords {
    fn new(x: usize, y: usize, t: usize) -> Self {
        Self {
            x: x as isize,
            y: y as isize,
            t: t as isize,
        }
    }
}

impl Display for Coords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.t)
    }
}

#[derive(Error, Debug)]
pub enum FxytError {
    #[error("RGB value greater than 255 or less than 0")]
    RgbOutOfRange,
    #[error("Attempt to push more than 8 values to the stack")]
    StackOverflow,
    #[error("Attempt to read from an empty stack")]
    StackEmpty,
    #[error("Attempt to divide by zero in mode 0")]
    DivideByZero,
    #[error("Attempt to increment mode beyond 2")]
    ModeOutOfRange,
    #[error("Failed to parse command")]
    Parse(#[from] ParseError),
    #[error("Debug command executed, output halted")]
    DebugHalt,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Found character that is not a valid FXYT command at position `{0}`")]
    InvalidCharacter(usize),
    #[error("Found a bracket with no partner at position `{0}`")]
    BracketMismatch(usize),
    #[error("Attempt to enter a loop more than 8 levels deep")]
    LoopNesting,
}

#[cfg(test)]
mod test {
    use rgb::{ComponentSlice, RGB8};
    use std::fs::File;
    use std::io::Write;

    #[test]
    #[ignore = "file i/o"]
    fn basic() {
        use crate::*;
        let output = render("XY^").unwrap();
        write_ppm(output[0]);
    }

    fn write_ppm(image_data: [[RGB8; 256]; 256]) {
        let mut file = File::create("output.ppm").unwrap();

        writeln!(file, "P6\n256 256\n255").unwrap();

        for row in image_data {
            for pixel in row {
                file.write_all(pixel.as_slice()).unwrap();
                // write!(file, "{}{}{}", pixel.r, pixel.g, pixel.b).unwrap()
            }
        }
    }

    #[test]
    fn loop_parsing() {
        use crate::*;
        use Command::*;
        let program = "NN5[N10+]";
        assert_eq!(
            (
                9,
                vec![
                    Integer,
                    Integer,
                    Digit(5),
                    Loop(vec![
                        Integer,
                        Digit(1),
                        Digit(0),
                        Arithmetic(crate::Arithmetic::Plus)
                    ])
                ]
            ),
            parse(program, 0, 0).unwrap()
        )
    }
    #[test]
    fn loop_parsing_doubly_nested() {
        use crate::*;
        use Command::*;
        let program = "NN5[N10[N4+]]";
        assert_eq!(
            (
                13,
                vec![
                    Integer,
                    Integer,
                    Digit(5),
                    Loop(vec![
                        Integer,
                        Digit(1),
                        Digit(0),
                        Loop(vec![Integer, Digit(4), Arithmetic(crate::Arithmetic::Plus)])
                    ])
                ]
            ),
            parse(program, 0, 0).unwrap()
        )
    }
}
