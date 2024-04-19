use crate::bindings::{
    fermyon::spin_test_virt,
    wasi::{http, io::streams},
};

impl PartialEq for spin_test_virt::key_value::Call {
    fn eq(&self, other: &Self) -> bool {
        use spin_test_virt::key_value::Call::*;
        match (self, other) {
            (Get(a), Get(b)) => a == b,
            (Set(a), Set(b)) => a == b,
            (Delete(a), Delete(b)) => a == b,
            (Exists(a), Exists(b)) => a == b,
            (GetKeys, GetKeys) => true,
            _ => false,
        }
    }
}

impl http::types::IncomingResponse {
    /// Read the body of the incoming response calling the callback on each chunk.
    pub fn read_body(self, callback: impl FnMut(Vec<u8>)) -> Result<(), streams::Error> {
        self.consume().unwrap().read(callback)
    }

    pub fn body_as_string(self) -> Result<String, streams::Error> {
        self.consume().unwrap().read_to_string()
    }
}

impl http::types::IncomingBody {
    /// Read the body of the incoming request calling the callback on each chunk.
    pub fn read(self, mut callback: impl FnMut(Vec<u8>)) -> Result<(), streams::Error> {
        struct Incoming(Option<(streams::InputStream, http::types::IncomingBody)>);

        impl Drop for Incoming {
            fn drop(&mut self) {
                if let Some((stream, body)) = self.0.take() {
                    drop(stream);
                    http::types::IncomingBody::finish(body);
                }
            }
        }

        let stream = self.stream().expect("response body should be readable");
        let pair = Incoming(Some((stream, self)));

        loop {
            if let Some((stream, _)) = &pair.0 {
                const READ_SIZE: u64 = 16 * 1024;
                match stream.blocking_read(READ_SIZE) {
                    Ok(buffer) => callback(buffer),
                    Err(streams::StreamError::Closed) => return Ok(()),
                    Err(streams::StreamError::LastOperationFailed(error)) => return Err(error),
                }
            }
        }
    }

    pub fn read_to_string(self) -> Result<String, streams::Error> {
        let mut result = String::new();
        self.read(|buffer| result.push_str(&String::from_utf8(buffer).unwrap()))?;
        Ok(result)
    }
}

impl http::types::OutgoingResponse {
    /// Get the body of the outgoing response.
    ///
    /// May only be called once.
    pub fn write_body(&self, bytes: &[u8]) {
        self.body().unwrap().write_bytes(bytes);
    }
}

impl http::types::OutgoingBody {
    /// Write bytes to the outgoing response body.
    pub fn write_bytes(self, mut bytes: &[u8]) {
        struct Outgoing(Option<(http::types::OutputStream, http::types::OutgoingBody)>);
        impl Outgoing {
            fn stream(&self) -> &http::types::OutputStream {
                &self.0.as_ref().unwrap().0
            }
        }

        impl Drop for Outgoing {
            fn drop(&mut self) {
                if let Some((stream, body)) = self.0.take() {
                    drop(stream);
                    _ = http::types::OutgoingBody::finish(body, None);
                }
            }
        }

        let stream = self.write().expect("response body should be writable");
        let pair = Outgoing(Some((stream, self)));

        let pollable = pair.stream().subscribe();
        while !bytes.is_empty() {
            // Block until ready to write
            pollable.block();
            // Check how much we can write
            let n = pair.stream().check_write().unwrap() as usize;
            // Get the minimum of how much we can write and how much we have left
            let len = std::cmp::min(n, bytes.len());
            // Break off the chunk we can write
            let (chunk, rest) = bytes.split_at(len);
            // Write the chunk
            pair.stream().write(chunk).unwrap();
            // Loop back with the rest
            bytes = rest;
        }
        // Flush the stream
        pair.stream().flush().unwrap();
        // Block until the stream is finished
        pollable.block();
    }
}

impl spin_test_virt::key_value::Store {
    pub fn calls(&self) -> Vec<spin_test_virt::key_value::Call> {
        spin_test_virt::key_value::calls()
            .iter()
            .find(|(store, _)| store == &self.label())
            .map(|(_, calls)| calls.clone())
            .unwrap_or_default()
    }
}
