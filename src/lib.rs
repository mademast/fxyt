use rgb::RGB8;
use thiserror::Error;

pub fn render(program: &str) -> Result<[[RGB8; 256]; 256], FxytError> {
    let parsed = parse(program)?;
    let mut canvas = [[RGB8::default(); 256]; 256];

    for x in 0..256 {
        #[allow(clippy::needless_range_loop)] //this is cleaner than what clippy wants
        for y in 0..256 {
            canvas[y][x] = render_pixel(&parsed, x, y, 0)?;
        }
    }

    Ok(canvas)
}

fn render_pixel(commands: &[Command], x: usize, y: usize, t: usize) -> Result<RGB8, FxytError> {
    let mut stack = Vec::with_capacity(8);
    let mut mode = 0;

    for command in commands {
        match command {
            Command::Coordinates(c) => match c {
                Coordinates::X => stack.push(x as isize),
                Coordinates::Y => stack.push(y as isize),
                Coordinates::T => stack.push(t as isize),
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
                                1 => return Ok(RGB8::default()),
                                2 => return Ok(RGB8::new(255, 0, 0)),
                                _ => unreachable!(),
                            }
                        }
                    }
                    Arithmetic::Modulus => left % right,
                })
            }
            Command::Mode => mode += 1,
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
            Command::Loop(inner_commands) => unimplemented!(),
            Command::FrameInterval => unimplemented!(),
            Command::Debug => {
                eprintln!("({x}, {y}) -> {:?}", stack);
                return Err(FxytError::DebugHalt);
            }
        }
        if stack.len() > 8 {
            return Err(FxytError::StackOverflow);
        }
        if mode > 2 {
            return Err(FxytError::ModeOutOfRange);
        }
    }

    let blue = stack.pop().unwrap_or_default();
    let green = stack.pop().unwrap_or_default();
    let red = stack.pop().unwrap_or_default();

    if red > 255 || green > 255 || blue > 255 || red < 0 || green < 0 || blue < 0 {
        return Err(FxytError::RgbOutOfRange);
    }

    Ok(RGB8::new(red as u8, green as u8, blue as u8))
}

fn parse(program: &str) -> Result<Vec<Command>, ParseError> {
    let mut parsed = Vec::with_capacity(program.len());
    let mut unparsed = program.chars();

    let mut index = 0;
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
            '[' => todo!(), //idea: recursion? add "current loop level" as a parameter to this function? ??
            ']' => todo!(),
            'F' => Command::FrameInterval,
            'W' => Command::Debug,

            _ => return Err(ParseError::InvalidCharacter(index)),
        };

        index += 1;

        parsed.push(next_command);
    }

    Ok(parsed)
}

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
    Loop(Vec<Command>), //TODO: parsing currently unimplemented due to loop difficulty
    FrameInterval,
    Debug,
}

enum Coordinates {
    X,
    Y,
    T,
}

enum Arithmetic {
    Plus,
    Minus,
    Times,
    Divide,
    Modulus,
}

enum Comparison {
    Equals,
    LessThan,
    GreaterThan,
}

enum Bitwise {
    Xor,
    And,
    Or,
}

enum StackOperation {
    Duplicate,
    Pop,
    Swap,
    Rotate,
}

#[derive(Error, Debug)]
pub enum FxytError {
    #[error("RGB value greater than 255 or less than 0")]
    RgbOutOfRange,
    #[error("Attempt to push more than 8 values to the stack")]
    StackOverflow,
    #[error("Attempt to read from an empty stack")]
    StackEmpty,
    #[error("Attempt to enter a loop more than 8 levels deep")]
    LoopNesting,
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
}

#[cfg(test)]
mod test {
    use rgb::{ComponentSlice, RGB8};
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn basic() {
        use crate::*;
        let output = render("XY^").unwrap();
        write_ppm(output);
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
}
