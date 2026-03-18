/// Adapter: `serialport::SerialPort` → `embedded_io::{Read, Write}`.
pub struct Serial(pub Box<dyn serialport::SerialPort>);

impl embedded_io::ErrorType for Serial {
    type Error = embedded_io::ErrorKind;
}

impl embedded_io::Read for Serial {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        std::io::Read::read(&mut self.0, buf).map_err(|_| embedded_io::ErrorKind::Other)
    }
}

impl embedded_io::Write for Serial {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        std::io::Write::write(&mut self.0, buf).map_err(|_| embedded_io::ErrorKind::Other)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        std::io::Write::flush(&mut self.0).map_err(|_| embedded_io::ErrorKind::Other)
    }
}
