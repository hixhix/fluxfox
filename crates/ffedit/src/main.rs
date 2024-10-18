/*
    ffedit
    https://github.com/dbalsom/fluxfox

    Copyright 2024 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    --------------------------------------------------------------------------
*/
mod app;
mod cmd_interpreter;
mod layout;
mod modal;

use core::fmt;
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use bpaf::{construct, short, OptionParser, Parser};

use ratatui::prelude::*;

use app::App;

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CmdParams {
    in_filename: Option<PathBuf>,
}

/// Set up bpaf argument parsing.
fn opts() -> OptionParser<CmdParams> {
    let in_filename = short('i')
        .long("in_filename")
        .help("Filename of image to read")
        .argument::<PathBuf>("IN_FILE")
        .optional();

    construct!(CmdParams { in_filename }).to_options()
}

// Define an enum for the history entries
enum HistoryEntry {
    UserCommand(String),
    CommandResponse(String),
}

#[derive(Default)]
pub struct DiskSelection {
    pub level: SelectionLevel,
    pub head: Option<u8>,
    pub cylinder: Option<u16>,
    pub sector: Option<u8>,
}

impl Display for DiskSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        match self.level {
            SelectionLevel::Disk => write!(f, ""),
            SelectionLevel::Head => write!(f, "[h:{}]", self.head.unwrap_or(0)),
            SelectionLevel::Track => write!(f, "[h:{} c:{}]", self.head.unwrap_or(0), self.cylinder.unwrap_or(0)),
            SelectionLevel::Sector => write!(
                f,
                "[h:{} c:{} s:{}]",
                self.head.unwrap_or(0),
                self.cylinder.unwrap_or(0),
                self.sector.unwrap_or(0)
            ),
        }
    }
}

/// Track the selection level
#[derive(Default)]
pub enum SelectionLevel {
    #[default]
    Disk,
    Head,
    Track,
    Sector,
}

const MAX_HISTORY: usize = 1000; // Maximum number of history entries

fn main() -> io::Result<()> {
    let opts = opts().run();
    let mut terminal = ratatui::init();
    let mut app = App::new(opts);
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}