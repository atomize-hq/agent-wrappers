use std::io::Read;

const CHUNK_SIZE_BYTES: usize = 8192;

#[derive(Debug)]
pub enum BoundedLine {
    Line {
        line_number: usize,
        bytes: Vec<u8>,
    },
    LineTooLong {
        line_number: usize,
        observed_bytes: usize,
        max_line_bytes: usize,
    },
    IoError {
        line_number: usize,
    },
}

pub struct SyncBoundedLineReader<R: Read> {
    reader: R,
    max_line_bytes: usize,
    buffer: [u8; CHUNK_SIZE_BYTES],
    buffer_pos: usize,
    buffer_len: usize,
    current_line: Vec<u8>,
    observed_bytes: usize,
    discard_mode: bool,
    line_number: usize,
    done: bool,
    pending_too_long: bool,
}

impl<R: Read> SyncBoundedLineReader<R> {
    pub fn new(reader: R, max_line_bytes: usize) -> Self {
        Self {
            reader,
            max_line_bytes,
            buffer: [0u8; CHUNK_SIZE_BYTES],
            buffer_pos: 0,
            buffer_len: 0,
            current_line: Vec::new(),
            observed_bytes: 0,
            discard_mode: false,
            line_number: 0,
            done: false,
            pending_too_long: false,
        }
    }

    fn fill_buffer(&mut self) -> Result<usize, ()> {
        self.buffer_pos = 0;
        match self.reader.read(&mut self.buffer) {
            Ok(n) => {
                self.buffer_len = n;
                Ok(n)
            }
            Err(_) => Err(()),
        }
    }

    fn finish_line(&mut self) -> BoundedLine {
        let line_number = self.line_number + 1;
        self.line_number = line_number;

        if self.pending_too_long {
            let observed_bytes = self.observed_bytes;
            let max_line_bytes = self.max_line_bytes;
            self.reset_line_state();
            return BoundedLine::LineTooLong {
                line_number,
                observed_bytes,
                max_line_bytes,
            };
        }

        let bytes = std::mem::take(&mut self.current_line);
        self.reset_line_state();
        BoundedLine::Line { line_number, bytes }
    }

    fn reset_line_state(&mut self) {
        self.current_line.clear();
        self.observed_bytes = 0;
        self.discard_mode = false;
        self.pending_too_long = false;
    }

    fn observe_bytes(&mut self, additional: usize) {
        self.observed_bytes = self.observed_bytes.saturating_add(additional);
        if self.observed_bytes > self.max_line_bytes && !self.discard_mode {
            self.discard_mode = true;
            self.pending_too_long = true;
            self.current_line.clear();
        }
    }
}

impl<R: Read> Iterator for SyncBoundedLineReader<R> {
    type Item = BoundedLine;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            if self.buffer_pos >= self.buffer_len {
                match self.fill_buffer() {
                    Ok(0) => {
                        self.done = true;
                        if self.pending_too_long || !self.current_line.is_empty() {
                            return Some(self.finish_line());
                        }
                        return None;
                    }
                    Ok(_) => {}
                    Err(()) => {
                        let line_number = self.line_number + 1;
                        self.line_number = line_number;
                        self.done = true;
                        return Some(BoundedLine::IoError { line_number });
                    }
                }
            }

            let (newline_idx, slice_len) = {
                let slice = &self.buffer[self.buffer_pos..self.buffer_len];
                (slice.iter().position(|b| *b == b'\n'), slice.len())
            };

            let Some(newline_idx) = newline_idx else {
                self.observe_bytes(slice_len);
                if !self.discard_mode {
                    let slice = &self.buffer[self.buffer_pos..self.buffer_len];
                    self.current_line.extend_from_slice(slice);
                }
                self.buffer_pos = self.buffer_len;
                continue;
            };

            self.observe_bytes(newline_idx);
            if !self.discard_mode {
                let segment = &self.buffer[self.buffer_pos..self.buffer_pos + newline_idx];
                self.current_line.extend_from_slice(segment);
            }
            self.buffer_pos = self.buffer_pos + newline_idx + 1;
            return Some(self.finish_line());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_line_is_discarded_and_iteration_continues() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ok\n");
        bytes.extend_from_slice(&vec![b'a'; 50]);
        bytes.extend_from_slice(b"\nnext\n");

        let reader = SyncBoundedLineReader::new(std::io::Cursor::new(bytes), 16);
        let lines: Vec<_> = reader.collect();

        assert!(matches!(lines[0], BoundedLine::Line { .. }));
        assert!(matches!(lines[1], BoundedLine::LineTooLong { .. }));
        assert!(matches!(lines[2], BoundedLine::Line { .. }));
    }
}
