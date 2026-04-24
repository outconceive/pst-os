pub const BLOCK_SIZE: usize = 512;

#[derive(Debug)]
pub enum BlockError {
    DeviceNotFound,
    IoError,
    InvalidBlock,
}

pub trait BlockDevice {
    fn read_block(&mut self, lba: u64, buf: &mut [u8; BLOCK_SIZE]) -> Result<(), BlockError>;
    fn write_block(&mut self, lba: u64, buf: &[u8; BLOCK_SIZE]) -> Result<(), BlockError>;
    fn capacity_blocks(&self) -> u64;
}
