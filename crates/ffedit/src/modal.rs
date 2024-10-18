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

// Modal state for the application
pub(crate) enum ModalState {
    ProgressBar(String, f64), // Title and completion percentage (0.0 to 1.0)
}

impl ModalState {
    // Create a new progress bar modal state
    pub(crate) fn new_progress_bar(title: &str) -> ModalState {
        ModalState::ProgressBar(title.to_string(), 0.0)
    }

    // Update the progress bar completion percentage
    pub(crate) fn update_progress(&mut self, percentage: f64) {
        if let ModalState::ProgressBar(_, ref mut p) = self {
            *p = percentage;
        }
    }

    pub(crate) fn input_enabled(&self) -> bool {
        match self {
            ModalState::ProgressBar(_, _) => false,
        }
    }
}