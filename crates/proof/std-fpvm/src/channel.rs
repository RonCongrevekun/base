//! This module contains a rudimentary channel between two file descriptors, using [`crate::io`]
//! for reading and writing from the file descriptors.
//!
//! [`read_exact`](FileChannel::read_exact) and [`write`](FileChannel::write) use synchronous
//! `io::read` / `io::write` loops inside `async fn` bodies with no `.await`. The resulting
//! futures complete on the first poll, matching the FPVM blocking syscall model and avoiding
//! `wake_by_ref` busy-wait loops that would spin the async executor.

use async_trait::async_trait;
use base_proof_preimage::{
    Channel,
    errors::{ChannelError, ChannelResult},
};

use crate::{FileDescriptor, io};

/// [`FileChannel`] is a handle for one end of a bidirectional channel.
#[derive(Debug, Clone, Copy)]
pub struct FileChannel {
    /// File descriptor to read from
    read_handle: FileDescriptor,
    /// File descriptor to write to
    write_handle: FileDescriptor,
}

impl FileChannel {
    /// Create a new [`FileChannel`] from two file descriptors.
    pub const fn new(read_handle: FileDescriptor, write_handle: FileDescriptor) -> Self {
        Self { read_handle, write_handle }
    }

    /// Returns a copy of the [`FileDescriptor`] used for the read end of the channel.
    pub const fn read_handle(&self) -> FileDescriptor {
        self.read_handle
    }

    /// Returns a copy of the [`FileDescriptor`] used for the write end of the channel.
    pub const fn write_handle(&self) -> FileDescriptor {
        self.write_handle
    }
}

#[async_trait]
impl Channel for FileChannel {
    async fn read(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        io::read(self.read_handle, buf).map_err(|_| ChannelError::Closed)
    }

    async fn read_exact(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        let mut total = 0;
        while total < buf.len() {
            let n =
                io::read(self.read_handle, &mut buf[total..]).map_err(|_| ChannelError::Closed)?;
            if n == 0 {
                return Err(ChannelError::Closed);
            }
            total += n;
        }
        Ok(total)
    }

    async fn write(&self, buf: &[u8]) -> ChannelResult<usize> {
        let mut written = 0;
        while written < buf.len() {
            let n =
                io::write(self.write_handle, &buf[written..]).map_err(|_| ChannelError::Closed)?;
            if n == 0 {
                return Err(ChannelError::Closed);
            }
            written += n;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_read_handle() {
        let read_handle = FileDescriptor::StdIn;
        let write_handle = FileDescriptor::StdOut;
        let chan = FileChannel::new(read_handle, write_handle);
        let ref_read_handle = chan.read_handle();
        assert_eq!(read_handle, ref_read_handle);
    }
}
