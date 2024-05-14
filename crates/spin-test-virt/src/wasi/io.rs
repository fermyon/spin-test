use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub use crate::bindings::{exports::wasi::io as exports, wasi::io as imports};
use crate::Component;

impl exports::error::Guest for Component {
    type Error = IoError;
}

pub struct IoError(String);

impl exports::error::GuestError for IoError {
    fn to_debug_string(&self) -> String {
        self.0.clone()
    }
}

impl exports::poll::Guest for Component {
    type Pollable = Pollable;

    fn poll(pollables: Vec<exports::poll::PollableBorrow<'_>>) -> Vec<u32> {
        // Keep track of the index of each host pollable...
        let mut index_to_host_polls = Vec::new();
        // ... and the index of each virtualized pollable
        let mut index_to_virt_polls = Vec::new();
        for (index, p) in pollables.iter().enumerate() {
            match p.get() {
                Pollable::Host(host) => {
                    index_to_host_polls.push((index, host));
                }
                Pollable::Virtualized => {
                    index_to_virt_polls.push(
                        index
                            .try_into()
                            .expect("found pollable with index > u32::MAX"),
                    );
                }
            }
        }

        // If we only have virtualized polls, we can just return all of them
        // since all virtualized polls are ready
        if !index_to_virt_polls.is_empty() && index_to_host_polls.is_empty() {
            return index_to_virt_polls;
        }

        // Poll all the host pollables
        let host_polls = index_to_host_polls
            .iter()
            .map(|(_, p)| *p)
            .collect::<Vec<_>>();
        let host_poll_results = imports::poll::poll(&host_polls);

        // If we have no virtualized polls, we can just delegate to the host
        if !index_to_host_polls.is_empty() && index_to_virt_polls.is_empty() {
            return host_poll_results;
        }

        // Otherwise, we have a mix of virtualized and host polls
        // Start with the virtualized polls
        let mut ready = index_to_virt_polls;

        // Extend with the host polls original indices
        let host_poll_results = host_poll_results.iter().map(|host_result| -> u32 {
            // `host_result` is the index of the host pollable that is ready
            let original_index = index_to_host_polls[*host_result as usize].0;
            original_index
                .try_into()
                .expect("found pollable with index > u32::MAX")
        });
        ready.extend(host_poll_results);
        ready
    }
}

pub enum Pollable {
    Host(imports::poll::Pollable),
    Virtualized,
}

impl exports::poll::GuestPollable for Pollable {
    fn ready(&self) -> bool {
        todo!()
    }

    fn block(&self) {
        match self {
            Pollable::Host(h) => h.block(),
            Pollable::Virtualized => {}
        }
    }
}

impl exports::streams::Guest for Component {
    type InputStream = InputStream;
    type OutputStream = OutputStream;
}

pub enum InputStream {
    Host(imports::streams::InputStream),
    Buffered(Buffer),
}

impl exports::streams::GuestInputStream for InputStream {
    fn read(&self, len: u64) -> Result<Vec<u8>, exports::streams::StreamError> {
        match self {
            InputStream::Host(h) => h.read(len).map_err(Into::into),
            InputStream::Buffered(buffer) if buffer.is_fully_read() => {
                Err(exports::streams::StreamError::Closed)
            }
            InputStream::Buffered(buffer) => Ok(buffer.read(len as usize).to_vec()),
        }
    }

    fn blocking_read(&self, len: u64) -> Result<Vec<u8>, exports::streams::StreamError> {
        match self {
            InputStream::Host(h) => h.blocking_read(len).map_err(Into::into),
            // Blocking streams have the same behavior as non-blocking
            InputStream::Buffered(_) => self.read(len),
        }
    }

    fn skip(&self, len: u64) -> Result<u64, exports::streams::StreamError> {
        todo!()
    }

    fn blocking_skip(&self, len: u64) -> Result<u64, exports::streams::StreamError> {
        todo!()
    }

    fn subscribe(&self) -> exports::streams::Pollable {
        let pollable = match self {
            InputStream::Host(stream) => {
                let pollable = imports::streams::InputStream::subscribe(stream);
                Pollable::Host(pollable)
            }
            // Buffered streams are always ready
            InputStream::Buffered(_) => Pollable::Virtualized,
        };
        exports::poll::Pollable::new(pollable)
    }
}

pub enum OutputStream {
    Host(imports::streams::OutputStream),
    Buffered(Buffer),
}

impl exports::streams::GuestOutputStream for OutputStream {
    fn check_write(&self) -> Result<u64, exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.check_write().map_err(Into::into),
            // Writers can always write as much as they want to a buffered stream
            OutputStream::Buffered(b) => Ok(usize::MAX as u64),
        }
    }

    fn write(&self, contents: Vec<u8>) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.write(&contents).map_err(Into::into),
            OutputStream::Buffered(b) => b.write(&contents),
        }
    }

    fn blocking_write_and_flush(
        &self,
        contents: Vec<u8>,
    ) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.blocking_write_and_flush(&contents).map_err(Into::into),
            // Blocking streams have the same behavior as non-blocking
            OutputStream::Buffered(_) => self.write(contents),
        }
    }

    fn flush(&self) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.flush().map_err(Into::into),
            OutputStream::Buffered(_) => Ok(()),
        }
    }

    fn blocking_flush(&self) -> Result<(), exports::streams::StreamError> {
        todo!()
    }

    fn subscribe(&self) -> exports::streams::Pollable {
        match self {
            OutputStream::Host(stream) => {
                let pollable = imports::streams::OutputStream::subscribe(stream);
                exports::poll::Pollable::new(Pollable::Host(pollable))
            }
            // Buffered streams are always ready
            OutputStream::Buffered(_) => exports::poll::Pollable::new(Pollable::Virtualized),
        }
    }

    fn write_zeroes(&self, len: u64) -> Result<(), exports::streams::StreamError> {
        todo!()
    }

    fn blocking_write_zeroes_and_flush(
        &self,
        len: u64,
    ) -> Result<(), exports::streams::StreamError> {
        todo!()
    }

    fn splice(
        &self,
        src: exports::streams::InputStreamBorrow<'_>,
        len: u64,
    ) -> Result<u64, exports::streams::StreamError> {
        todo!()
    }

    fn blocking_splice(
        &self,
        src: exports::streams::InputStreamBorrow<'_>,
        len: u64,
    ) -> Result<u64, exports::streams::StreamError> {
        todo!()
    }
}

impl From<imports::streams::StreamError> for exports::streams::StreamError {
    fn from(e: imports::streams::StreamError) -> Self {
        match e {
            imports::streams::StreamError::Closed => exports::streams::StreamError::Closed,
            imports::streams::StreamError::LastOperationFailed(e) => {
                exports::streams::StreamError::LastOperationFailed(exports::error::Error::new(
                    IoError(e.to_debug_string()),
                ))
            }
        }
    }
}

// A simple buffer that can be used as a stream
#[derive(Clone, Debug)]
pub struct Buffer {
    inner: Rc<RefCell<Vec<u8>>>,
    read_offset: Cell<usize>,
}

impl Buffer {
    /// Create a new buffer with the given contents
    pub fn new(inner: Vec<u8>) -> Self {
        Buffer {
            inner: Rc::new(RefCell::new(inner)),
            read_offset: Cell::new(0),
        }
    }

    pub fn empty() -> Buffer {
        Self::new(Default::default())
    }

    /// Read the next `len` bytes from the buffer
    fn read(&self, len: usize) -> impl std::ops::Deref<Target = [u8]> + '_ {
        let end = std::cmp::min(self.read_offset.get() + len, self.inner.borrow().len());
        let slice = std::cell::Ref::map(self.inner.borrow(), |s| &s[self.read_offset.get()..end]);
        self.read_offset.set(end);
        slice
    }

    /// Write the given contents to the buffer
    fn write(&self, contents: &[u8]) -> Result<(), exports::streams::StreamError> {
        Ok(self.inner.borrow_mut().extend_from_slice(contents))
    }

    /// Check if the buffer has been fully read
    fn is_fully_read(&self) -> bool {
        self.inner.borrow().len() == self.read_offset.get()
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(v: Vec<u8>) -> Self {
        Buffer::new(v)
    }
}

impl From<String> for Buffer {
    fn from(v: String) -> Self {
        Buffer::new(v.into_bytes())
    }
}
