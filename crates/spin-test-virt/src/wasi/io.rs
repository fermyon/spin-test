#![allow(unused_variables)]
use crate::bindings::{self, wasi::cli::stdout::get_stdout};
pub use crate::bindings::{exports::wasi::io as exports, wasi::io as imports};
use crate::Component;

impl exports::error::Guest for Component {
    type Error = IoError;
}

pub struct IoError;

impl exports::error::GuestError for IoError {
    fn to_debug_string(&self) -> String {
        todo!()
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
    Virtualized,
}

impl exports::streams::GuestInputStream for InputStream {
    fn read(&self, len: u64) -> Result<Vec<u8>, exports::streams::StreamError> {
        match self {
            InputStream::Host(h) => h.read(len).map_err(Into::into),
            // Virtualized streams are always done
            InputStream::Virtualized => Err(exports::streams::StreamError::Closed),
        }
    }

    fn blocking_read(&self, len: u64) -> Result<Vec<u8>, exports::streams::StreamError> {
        match self {
            InputStream::Host(h) => h.blocking_read(len).map_err(Into::into),
            InputStream::Virtualized => Ok(Vec::new()),
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
            InputStream::Virtualized => Pollable::Virtualized,
        };
        exports::poll::Pollable::new(pollable)
    }
}

pub enum OutputStream {
    Host(imports::streams::OutputStream),
    Virtualized,
}

impl exports::streams::GuestOutputStream for OutputStream {
    fn check_write(&self) -> Result<u64, exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.check_write().map_err(Into::into),
            OutputStream::Virtualized => Ok(u64::MAX),
        }
    }

    fn write(&self, contents: Vec<u8>) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.write(&contents).map_err(Into::into),
            OutputStream::Virtualized => Ok(()),
        }
    }

    fn blocking_write_and_flush(
        &self,
        contents: Vec<u8>,
    ) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.blocking_write_and_flush(&contents).map_err(Into::into),
            OutputStream::Virtualized => Ok(()),
        }
    }

    fn flush(&self) -> Result<(), exports::streams::StreamError> {
        match self {
            OutputStream::Host(h) => h.flush().map_err(Into::into),
            OutputStream::Virtualized => Ok(()),
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
            OutputStream::Virtualized => exports::poll::Pollable::new(Pollable::Virtualized),
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
                    IoError,
                ))
            }
        }
    }
}
