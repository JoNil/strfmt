// #![feature(test)]
// #[cfg_attr(test, feature(test))]
// extern crate test;

// use std::io::Write;
use std::fmt;
use std::error;
use std::fmt::Write;
use std::iter::Iterator;
use std::collections::HashMap;
use std::string::String;

extern crate regex;
#[macro_use]
extern crate lazy_static;
use regex::Regex;

#[cfg(test)]
mod tests;

lazy_static!{
    pub static ref FMT_PAT: Regex = Regex::new(
//        1-ident 2-fill 3-align 4-width  5-precision
        r"^([\w\d-_]+)(?::(.)?([<>^])?([\d]+)?(?:\.([\d]+))?)?\z").unwrap();
// if align doesn't exist, width == fill + width
}

#[derive(Debug, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
    None,
}

#[derive(Debug, PartialEq)]
pub struct FmtError(String);

impl fmt::Display for FmtError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FmtError({:?})", self.0)
    }
}

impl error::Error for FmtError {
    fn description(&self) -> &str {
        "fmt error"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// LOC-error
#[derive(Debug, PartialEq)]
struct Fmt<'a> {
    pub identifier: &'a str,
    pub fill: Option<char>,
    pub align: Align,
    pub width: Option<usize>,
    pub precision: Option<usize>,
}

fn write_char(s: &mut String, c: char, n: usize) {
    for _ in 0..n {
        s.push(c);
    }
}

#[test]
fn test_write_char() {
    let mut s = String::new();
    s.write_str("h ").unwrap();
    write_char(&mut s, 'f', 3);
    assert!(s == "h fff");
}

fn write_from<'a, I>(s: &mut String, f: I, n: usize) -> usize
    where I: Iterator<Item = char>
{
    // eexaust f or run out of n, return chars written
    if n == 0 {
        return 0;
    }
    let mut n_written: usize = 0;
    for c in f {
        s.push(c);
        n_written += 1;
        if n_written == n {
            return n_written;
        }
    }
    n_written
}

#[test]
fn test_write_from() {
    let mut s = String::new();
    s.write_str("h ").unwrap();
    write_from(&mut s, "fff".chars(), 5);
    assert!(s == "h fff");
    write_from(&mut s, "xxxx".chars(), 2);
    assert!(s == "h fffxx");
    write_from(&mut s, "333".chars(), 3);
    assert!(s == "h fffxx333");
}

impl<'a> Fmt<'a> {
    /// create Fmt from format string
    pub fn from_str(fmt: &'a str) -> Result<Fmt, String> {
        let captures = match FMT_PAT.captures(fmt) {
            None => return Err("Invalid format string".to_string()),
            Some(c) => c,
        };
        // println!("captures: {:?}", captures);
        let mut out = Fmt {
            identifier: captures.at(1).unwrap(), // not optional==unwrap
            fill: None,
            align: Align::None,
            width: None,
            precision: None,
        };

        let fake_fill = match captures.at(2) {
            None => return Ok(out), // no characters after ':', just return
            Some(f) => f,
        };

        out.align = match captures.at(3) {
            None => Align::None,
            Some("<") => Align::Left,
            Some("^") => Align::Center,
            Some(">") => Align::Right,
            _ => unreachable!(),
        };

        if out.align != Align::None {
            // simple case where everything equals what it should
            out.fill = Some(fake_fill.chars().next().unwrap());
            out.width = match captures.at(4) {
                None => None,
                Some(width) => Some(width.parse::<usize>().unwrap()),
            };
            out.precision = match captures.at(5) {
                None => None,
                Some(prec) => Some(prec.parse::<usize>().unwrap()),
            };
            return Ok(out);
        }

        let fake_width: Option<&str> = captures.at(4);
        let fake_precision: Option<&str> = captures.at(5);

        // we know that align is None

        // if width is not none and fill == '.' then precision == width, width == None
        if fake_width != None && fake_fill == "." {
            out.precision = Some(fake_width.unwrap().parse::<usize>().unwrap());
            return Ok(out);
        }
        let fake_align = match fake_fill {
            "<" => Align::Left,
            "^" => Align::Center,
            ">" => Align::Right,
            _ => Align::None,
        };
        if fake_align != Align::None {
            out.align = fake_align;
            out.width = match fake_width {
                None => None,
                Some(x) => Some(x.parse::<usize>().unwrap()),
            };
            out.precision = match fake_precision {
                None => None,
                Some(x) => Some(x.parse::<usize>().unwrap()),
            };
            return Ok(out);
        }
        // now we know that width == fake_fill + fake_width
        let mut wstr = String::new();
        wstr.write_str(fake_fill).unwrap();
        if fake_width != None {
            wstr.write_str(fake_width.unwrap()).unwrap();
        }
        out.width = match wstr.parse::<usize>() {
            Ok(w) => Some(w),
            Err(_) => return Err("invalid width: must be an int".to_string()),
        };
        out.precision = match fake_precision {
            None => None,
            Some(x) => Some(x.parse::<usize>().unwrap()),
        };
        Ok(out)
    }

    /// write the formatted string to `s` and return true. If there is an error: clear `s`,
    /// write the error and return false
    pub fn write(&self, s: &mut String, vars: &'a HashMap<String, String>) -> bool {
        // println!("- writting...");
        let ref value = match vars.get(self.identifier) {
            Some(v) => v,
            None => {
                s.clear();
                write!(s, "invalid identifier: {}", self.identifier).unwrap();
                return false;
            }
        };
        let len = match self.precision {
            None => value.len(),
            Some(p) => {
                if p < value.len() {
                    p
                } else {
                    value.len()
                }
            }
        };
        let mut value = value.chars();
        let mut pad: usize = 0;
        let fill = self.fill.unwrap_or(' ');

        match self.width {
            Some(mut width) => {
                if width > len {
                    // println!("  - width > len");
                    match self.align {
                        Align::Left => pad = width - len,
                        Align::Center => {
                            width = width - len;
                            pad = width / 2;
                            write_char(s, fill, pad);
                            pad += width % 2;
                        }
                        Align::Right | Align::None => {
                            write_char(s, fill, width - len);
                        }
                    }
                }
            }
            None => {}
        }
        if self.precision.is_none() {
            s.extend(value);
        } else {
            write_from(s, &mut value, self.precision.unwrap());
        }
        write_char(s, fill, pad);
        true
    }
}

/// rust-style format a string given a HashMap of the variables
pub fn strfmt(fmtstr: &str, vars: &HashMap<String, String>) -> Result<String, FmtError> {
    let mut out = String::with_capacity(fmtstr.len() * 2);
    let mut bytes_read: usize = 0;
    let mut opening_brace: usize = 0;
    let mut reading_fmt = false;
    let mut remaining = fmtstr;
    let mut last_was_discarded_brace = false;
    for c in fmtstr.chars() {
        bytes_read += c.len_utf8();
        if c == '{' {
            if reading_fmt && opening_brace == bytes_read - 2 {
                // found {{
                out.push(c);
                reading_fmt = false;
            } else if !reading_fmt {
                // found a first {
                reading_fmt = true;
                opening_brace = bytes_read - 1;
            } else {
                // found a { after finding an opening brace, error!
                out.clear();
                out.write_str("extra { found").unwrap();
                return Err(FmtError(out));
            }
        } else if c == '}' {
            if !reading_fmt {
                if !last_was_discarded_brace {
                    out.push(c); // extra '}' found, ignore
                    last_was_discarded_brace = true;
                } else {
                    last_was_discarded_brace = false;
                }
            } else {
                // discard before opening brace
                // println!(" - remaining before: {:?}", remaining);
                let (_, r) = remaining.split_at(opening_brace);

                // get the fmt pattern and remaining
                let (fmt_pattern, r) = r.split_at(bytes_read - opening_brace);
                remaining = r;

                // discard the braces
                let (_, fmt_pattern) = fmt_pattern.split_at(1);
                let (fmt_pattern, _) = fmt_pattern.split_at(fmt_pattern.len() - 1);
                // println!(" - pattern found: {:?}", fmt_pattern);
                // println!(" - remaining after: {:?}", remaining);
                // use the Fmt object to write the formatted string
                match Fmt::from_str(fmt_pattern) {
                    Ok(fmt) => {
                        match fmt.write(&mut out, vars) {
                            true => {}
                            false => {
                                return Err(FmtError(out));
                            }
                        }
                    }
                    Err(err) => return Err(FmtError(err)),
                };
                reading_fmt = false;
                bytes_read = 0;
                last_was_discarded_brace = false;
            }
        } else if !reading_fmt {
            out.push(c)
        } // else we are currently reading a format string, so don't push
    }
    out.shrink_to_fit();
    Ok(out)
}
