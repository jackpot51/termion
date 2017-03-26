//! Cursor movement.

use std::fmt;
use std::io::{self, Read, Write, Error, ErrorKind};
use async::async_stdin;
use std::time::{SystemTime, Duration};
use raw::CONTROL_SEQUENCE_TIMEOUT;
use sys::tty;

derive_csi_sequence!("Hide the cursor.", Hide, "?25l");
derive_csi_sequence!("Show the cursor.", Show, "?25h");

derive_csi_sequence!("Restore the cursor.", Restore, "u");
derive_csi_sequence!("Save the cursor.", Save, "s");

/// Goto some position ((1,1)-based).
///
/// # Why one-based?
///
/// ANSI escapes are very poorly designed, and one of the many odd aspects is being one-based. This
/// can be quite strange at first, but it is not that big of an obstruction once you get used to
/// it.
///
/// # Example
///
/// ```rust
/// extern crate termion;
///
/// fn main() {
///     print!("{}{}Stuff", termion::clear::All, termion::cursor::Goto(5, 3));
/// }
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Goto(pub u16, pub u16);

impl Default for Goto {
    fn default() -> Goto {
        Goto(1, 1)
    }
}

impl fmt::Display for Goto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug_assert!(self != &Goto(0, 0), "Goto is one-based.");

        write!(f, csi!("{};{}H"), self.1, self.0)
    }
}

/// Move cursor left.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Left(pub u16);

impl fmt::Display for Left {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}D"), self.0)
    }
}

/// Move cursor right.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Right(pub u16);

impl fmt::Display for Right {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}C"), self.0)
    }
}

/// Move cursor up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Up(pub u16);

impl fmt::Display for Up {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}A"), self.0)
    }
}

/// Move cursor down.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Down(pub u16);

impl fmt::Display for Down {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}B"), self.0)
    }
}

/// Types that allow detection of the cursor position.
pub trait DetectCursorPos {
    /// Get the (1,1)-based cursor position from the terminal.
    fn cursor_pos(&mut self) -> io::Result<(u16, u16)>;
}

pub enum AnsiState {
    Norm,
    Esc,
    Csi,
    Osc,
}

impl<W: Write> DetectCursorPos for W {
    fn cursor_pos(&mut self) -> io::Result<(u16, u16)> {
        let mut stdin = tty::get_tty()?;

        write!(self, "\x1B[6n")?;
        self.flush()?;

        let mut arg = String::new();
        let mut s = AnsiState::Norm;
        for b_res in stdin.bytes() {
            let b = b_res?;
            match s {
                AnsiState::Norm => match b {
                    b'\x1B' => s = AnsiState::Esc,
                    _ => (),
                },
                AnsiState::Esc => match b {
                    b'[' => {
                        arg.clear();
                        s = AnsiState::Csi;
                    },
                    b']' => s = AnsiState::Osc,
                    _ => s = AnsiState::Norm,
                },
                AnsiState::Csi => match b {
                    b'R' => {
                        let mut parts = arg.split(';');
                        let y = parts.next().unwrap_or("").parse::<u16>().unwrap_or(0);
                        let x = parts.next().unwrap_or("").parse::<u16>().unwrap_or(0);
                        return Ok((x, y));
                    },
                    b'A' ... b'Z' | b'a' ... b'z' => s = AnsiState::Norm,
                    b'0' ... b'9' | b';' => arg.push(b as char),
                    _ => ()
                },
                AnsiState::Osc => match b {
                    b'\x07' => s = AnsiState::Norm,
                    _ => (),
                }
            }
        }

        Err(Error::new(ErrorKind::Other, "Cursor position not found"))
    }
}
