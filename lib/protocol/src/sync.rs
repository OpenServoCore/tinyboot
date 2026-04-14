use crate::ReadError;

/// Frame sync preamble.
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct Sync(pub(crate) u8, pub(crate) u8);

impl Sync {
    /// Valid sync word for building frames.
    pub const fn valid() -> Self {
        Self(0xAA, 0x55)
    }

    /// Block until the sync word is received, updating self.
    pub fn read<R: embedded_io::Read>(&mut self, r: &mut R) -> Result<(), ReadError> {
        let mut b = [0u8; 1];
        loop {
            r.read(&mut b).map_err(|_| ReadError)?;
            self.0 = self.1;
            self.1 = b[0];
            if self.0 == Self::valid().0 && self.1 == Self::valid().1 {
                return Ok(());
            }
        }
    }

    /// Async version of read.
    pub async fn read_async<R: embedded_io_async::Read>(
        &mut self,
        r: &mut R,
    ) -> Result<(), ReadError> {
        let mut b = [0u8; 1];
        loop {
            r.read(&mut b).await.map_err(|_| ReadError)?;
            self.0 = self.1;
            self.1 = b[0];
            if self.0 == Self::valid().0 && self.1 == Self::valid().1 {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockReader<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl<'a> MockReader<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self { data, pos: 0 }
        }
    }

    impl embedded_io::ErrorType for MockReader<'_> {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::Read for MockReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let n = buf.len().min(self.data.len() - self.pos);
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    #[test]
    fn sync_immediate() {
        let mut sync = Sync::default();
        sync.read(&mut MockReader::new(&[0xAA, 0x55])).unwrap();
        assert_eq!(sync, Sync::valid());
    }

    #[test]
    fn sync_after_garbage() {
        let mut sync = Sync::default();
        sync.read(&mut MockReader::new(&[0xFF, 0x00, 0x42, 0xAA, 0x55]))
            .unwrap();
        assert_eq!(sync, Sync::valid());
    }

    #[test]
    fn sync_false_start() {
        let mut sync = Sync::default();
        sync.read(&mut MockReader::new(&[0xAA, 0x00, 0xAA, 0x55]))
            .unwrap();
        assert_eq!(sync, Sync::valid());
    }

    #[test]
    fn sync_repeated_first_byte() {
        let mut sync = Sync::default();
        sync.read(&mut MockReader::new(&[0xAA, 0xAA, 0x55]))
            .unwrap();
        assert_eq!(sync, Sync::valid());
    }

    #[test]
    fn default_is_invalid() {
        assert_ne!(Sync::default(), Sync::valid());
    }
}
