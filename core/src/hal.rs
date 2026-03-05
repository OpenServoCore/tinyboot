pub trait ChipHal {
    // UART
    fn uart_read_byte(&mut self) -> u8;
    fn uart_write_byte(&mut self, byte: u8);

    // Flash
    fn flash_erase_page(&mut self, addr: u32);
    fn flash_write_page(&mut self, addr: u32, data: &[u8; 64]);

    // System
    fn system_reset(&mut self) -> !;
}
