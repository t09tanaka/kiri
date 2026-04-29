//! Per-terminal monotonic ring buffer used by `Read` / `Follow`.
//!
//! Bytes are pushed in order. When the buffer is full, the oldest bytes
//! are evicted to make room. A monotonically increasing `cursor` tracks
//! the total number of bytes ever pushed so callers can request "give me
//! everything since byte N" semantics. If N is older than the oldest
//! retained byte, callers learn how many bytes were dropped.

use std::collections::VecDeque;

pub struct RingBuffer {
    buf: VecDeque<u8>,
    cap: usize,
    /// Total bytes ever pushed (never decreases).
    cursor: u64,
}

impl RingBuffer {
    pub fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(cap),
            cap,
            cursor: 0,
        }
    }

    /// Append `data` to the buffer, evicting oldest bytes when capacity is
    /// exceeded. Always advances `cursor` by `data.len()`.
    pub fn push(&mut self, data: &[u8]) {
        // If `data` alone is larger than the capacity, only the tail
        // `cap` bytes can be retained — anything earlier is evicted
        // before it ever lands in the buffer.
        let to_store: &[u8] = if data.len() > self.cap {
            &data[data.len() - self.cap..]
        } else {
            data
        };

        // Make room for `to_store.len()` new bytes.
        let needed = to_store.len();
        let available = self.cap - self.buf.len();
        if needed > available {
            let to_drop = needed - available;
            for _ in 0..to_drop {
                self.buf.pop_front();
            }
        }
        self.buf.extend(to_store.iter().copied());
        self.cursor += data.len() as u64;
    }

    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    /// Returns `(bytes, bytes_dropped)`.
    /// - If `since >= cursor` the result is `(empty, 0)` (nothing new).
    /// - If `since < oldest_kept` then `bytes_dropped = oldest_kept - since`
    ///   and the returned bytes start at `oldest_kept`.
    /// - Otherwise the bytes start at exactly `since`.
    pub fn read_since(&self, since: u64) -> (Vec<u8>, u64) {
        if since >= self.cursor {
            return (Vec::new(), 0);
        }
        let buffered = self.buf.len() as u64;
        let oldest_kept = self.cursor - buffered;
        let (start_byte, dropped) = if since < oldest_kept {
            (oldest_kept, oldest_kept - since)
        } else {
            (since, 0)
        };
        let offset = (start_byte - oldest_kept) as usize;
        let bytes: Vec<u8> = self.buf.iter().copied().skip(offset).collect();
        (bytes, dropped)
    }

    /// Returns the last `n` newline-terminated chunks plus the number of
    /// lines omitted from the start. A trailing partial line counts as
    /// one chunk.
    ///
    /// `n == 0` yields `(empty, total_lines)`.
    pub fn tail_lines(&self, n: usize) -> (Vec<u8>, usize) {
        let bytes: Vec<u8> = self.buf.iter().copied().collect();
        let chunks: Vec<&[u8]> = split_inclusive_newline(&bytes);
        let total = chunks.len();
        if n == 0 {
            return (Vec::new(), total);
        }
        if n >= total {
            return (bytes, 0);
        }
        let omitted = total - n;
        let mut out: Vec<u8> = Vec::new();
        for chunk in chunks.iter().skip(omitted) {
            out.extend_from_slice(chunk);
        }
        (out, omitted)
    }
}

/// `[u8]::split_inclusive(|b| *b == b'\n')` returns an iterator of slices
/// in stdlib, but we need it to work over a `&[u8]` produced from a
/// `VecDeque` via `Vec::from_iter`. This helper just collects the slices.
fn split_inclusive_newline(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            out.push(&bytes[start..=i]);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        out.push(&bytes[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_read_full() {
        let mut rb = RingBuffer::new(16);
        rb.push(b"hello");
        let (bytes, dropped) = rb.read_since(0);
        assert_eq!(bytes, b"hello");
        assert_eq!(dropped, 0);
        assert_eq!(rb.cursor(), 5);
    }

    #[test]
    fn drops_when_capacity_exceeded() {
        let mut rb = RingBuffer::new(4);
        rb.push(b"abcdef");
        let (bytes, dropped) = rb.read_since(0);
        assert_eq!(bytes, b"cdef");
        assert_eq!(dropped, 2);
        assert_eq!(rb.cursor(), 6);
    }

    #[test]
    fn read_since_in_window() {
        let mut rb = RingBuffer::new(8);
        rb.push(b"abcdef");
        let (bytes, dropped) = rb.read_since(3);
        assert_eq!(bytes, b"def");
        assert_eq!(dropped, 0);
    }

    #[test]
    fn read_since_future_returns_empty() {
        let mut rb = RingBuffer::new(8);
        rb.push(b"abc");
        let (bytes, dropped) = rb.read_since(99);
        assert!(bytes.is_empty());
        assert_eq!(dropped, 0);
    }

    #[test]
    fn tail_lines_returns_last_n() {
        let mut rb = RingBuffer::new(64);
        rb.push(b"one\ntwo\nthree\nfour\n");
        let (bytes, omitted) = rb.tail_lines(2);
        assert_eq!(bytes, b"three\nfour\n");
        assert_eq!(omitted, 2);
    }

    #[test]
    fn tail_lines_more_than_available_returns_all() {
        let mut rb = RingBuffer::new(64);
        rb.push(b"a\nb\n");
        let (bytes, omitted) = rb.tail_lines(10);
        assert_eq!(bytes, b"a\nb\n");
        assert_eq!(omitted, 0);
    }
}
