use core::slice;
use std::{
    ffi::{CStr, CString}, io::{self, Write}, os::raw::c_char
};

use lazy_static::lazy_static;
use sharded_slab::{pool::RefMut, Pool};
// use smallvec::SmallVec;
// use tracing_core::Metadata;
use tracing_subscriber::fmt::MakeWriter;
use godot::prelude::*;

// use crate::logging::{Buffer, Priority};

/// The writer produced by [`GodotWasmLogMakeWriter`].
#[derive(Debug)]
pub struct GodotWasmLogWriter<'a> {
    tag: &'a CStr,
    message: PooledCString,
    location: Option<Location>,
}

/// A [`MakeWriter`] suitable for writing Android logs.
#[derive(Debug)]
pub struct GodotWasmLogMakeWriter {
    tag: CString,
}

#[derive(Debug)]
struct Location {
    file: PooledCString,
    line: u32,
}

// logd truncates logs at 4096 bytes, so we chunk at 4000 to be conservative
const MAX_LOG_LEN: usize = 4000;

impl Write for GodotWasmLogWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.message.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Convert the message to a string, fallback to empty string if invalid UTF-8
        let message = String::from_utf8_lossy(self.message.as_bytes());
        
        // Create location string if available
        let location_str = self.location.as_ref().map(|loc| {
            let file = String::from_utf8_lossy(loc.file.as_bytes());
            format!(" ({}:{})", file, loc.line)
        }).unwrap_or_default();

        // Combine message and location
        let full_message = format!("{}{}", message, location_str);

        // Use godot::log::print
        godot::log::print(&[full_message.to_variant()]);

        Ok(())
    }
}

impl Drop for GodotWasmLogWriter<'_> {
    fn drop(&mut self) {
        self.flush().unwrap();
    }
}

impl<'a> MakeWriter<'a> for GodotWasmLogMakeWriter {
    type Writer = GodotWasmLogWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        GodotWasmLogWriter {
            tag: self.tag.as_c_str(),
            message: PooledCString::empty(),

            location: None,
        }
    }

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {

        let location = match (meta.file(), meta.line()) {
            (Some(file), Some(line)) => {
                let file = PooledCString::new(file.as_bytes());
                Some(Location { file, line })
            }
            _ => None,
        };

        GodotWasmLogWriter {
            tag: self.tag.as_c_str(),
            message: PooledCString::empty(),
            location,
        }
    }
}

impl GodotWasmLogMakeWriter {
    /// Returns a new [`GodotWasmLogWriter`] with the given tag.
    pub fn new(tag: String) -> Self {
        Self {
            tag: CString::new(tag).unwrap(),
        }
    }
}

#[derive(Debug)]
struct PooledCString {
    buf: RefMut<'static, Vec<u8>>,
}

enum MessageIter<'a> {
    Single(Option<&'a mut PooledCString>),
    Multi(slice::IterMut<'a, PooledCString>),
}

lazy_static! {
    static ref BUFFER_POOL: Pool<Vec<u8>> = Pool::new();
}

impl PooledCString {
    fn empty() -> Self {
        Self {
            buf: BUFFER_POOL.create().unwrap(),
        }
    }

    fn new(data: &[u8]) -> Self {
        let mut this = PooledCString::empty();
        this.write(data);
        this
    }

    fn write(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    fn as_ptr(&mut self) -> Option<*const c_char> {
        if self.buf.last().copied() != Some(0) {
            self.buf.push(0);
        }

        CStr::from_bytes_with_nul(self.buf.as_ref())
            .ok()
            .map(CStr::as_ptr)
    }

    fn as_bytes(&self) -> &[u8] {
        self.buf.as_ref()
    }
}

impl Drop for PooledCString {
    fn drop(&mut self) {
        BUFFER_POOL.clear(self.buf.key());
    }
}

impl<'a> Iterator for MessageIter<'a> {
    type Item = &'a mut PooledCString;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MessageIter::Single(message) => message.take(),
            MessageIter::Multi(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            MessageIter::Single(Some(_)) => (1, Some(1)),
            MessageIter::Single(None) => (0, Some(0)),
            MessageIter::Multi(iter) => iter.size_hint(),
        }
    }
}