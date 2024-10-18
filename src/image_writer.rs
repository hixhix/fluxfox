/*
    FluxFox
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

    src/image_writer.rs

    Implements an output helper for writing disk images to a file.

*/
use crate::io::Cursor;
use crate::{DiskImage, DiskImageError, DiskImageFormat, ImageParser};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ImageWriter {
    pub path: Option<PathBuf>,
    pub format: Option<DiskImageFormat>,
}

impl ImageWriter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_format(self, format: DiskImageFormat) -> Self {
        Self {
            format: Some(format),
            ..self
        }
    }

    pub fn with_path(self, path: PathBuf) -> Self {
        Self {
            path: Some(path),
            ..self
        }
    }

    pub fn write(self, image: &mut DiskImage) -> Result<(), DiskImageError> {
        if self.path.is_none() {
            return Err(DiskImageError::ParameterError);
        }
        if self.format.is_none() {
            return Err(DiskImageError::ParameterError);
        }

        let path = self.path.unwrap();
        let format = self.format.unwrap();

        let mut buf = Cursor::new(Vec::with_capacity(1_000_000));

        format.save_image(image, &mut buf)?;

        let data = buf.into_inner();
        std::fs::write(path, data)?;

        Ok(())
    }
}