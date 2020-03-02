use rust_decimal::{Decimal,Error};
use std::str::FromStr;
use bitpacking::{BitPacker8x, BitPacker};

pub struct Packer {
    bitpacker: BitPacker8x,
    buffer: Vec<[u32; 4]>,
    compressed: [Vec<u32>; 4],
    packed: [Vec<u8>; 4],
    trim: bool,
}

impl Packer {
    pub fn new() -> Packer {
        Packer {
            bitpacker: BitPacker8x::new(),
            buffer: Vec::new(),
            compressed: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            packed: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            trim: true,
        }
    }

    pub fn with_trim(&mut self, trim: bool) -> &mut Self {
        self.trim = trim;
        self
    }

    pub fn load(&mut self, value: &str) -> Result<(),Error> {
        let result;
        if self.trim {
            result = value.trim_matches('0');
        } else {
            result = value;
        }
        Ok(
            self.load_decimal(Decimal::from_str(result)?)
        )
    }
    
    pub fn load_decimal(&mut self, value: Decimal) {
        let parsed = zip_u8(value.serialize());
        match self.buffer.last() {
            Some(last) => {
                self.compressed[0].push(parsed[0] ^ last[0]);
                self.compressed[1].push(parsed[1] ^ last[1]);
                self.compressed[2].push(parsed[2] ^ last[2]);
                self.compressed[3].push(parsed[3] ^ last[3]);
            },
            None => {
                self.compressed[0].push(parsed[0]);
                self.compressed[1].push(parsed[1]);
                self.compressed[2].push(parsed[2]);
                self.compressed[3].push(parsed[3]);
            }
        }
        self.buffer.push(parsed);

        if self.compressed[0].len() == 256 {
            self.pack()
        }
    }

    pub fn print(&self) {
        println!("{:?}", self.packed)
    }

    fn pack(&mut self) {
        for i in 0..4 {
            let num_bits = self.bitpacker.num_bits(&self.compressed[i]);
            let mut compressed = vec![0u8; (num_bits as usize) * BitPacker8x::BLOCK_LEN / 8];

            let _ = self.bitpacker.compress(&self.compressed[i], &mut compressed[..], num_bits);
            self.packed[i].append(&mut compressed.to_vec());

            // Decompressing
            // let mut decompressed = vec![0u32; BitPacker8x::BLOCK_LEN];
            // self.bitpacker.decompress(&compressed[..compressed_len], &mut decompressed[..], num_bits);

            // assert_eq!(&self.compressed[i], &decompressed);
        }
        self.flush_cache()
    }

    fn flush_cache(&mut self) {
        self.buffer = Vec::new();
        self.compressed = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    }
}

fn zip_u8(values: [u8; 16]) -> [u32; 4] {
    [
        u32::from_be_bytes([values[0], values[1], values[2], values[3]]),
        u32::from_le_bytes([values[4], values[5], values[6], values[7]]),
        u32::from_le_bytes([values[8], values[9], values[10], values[11]]),
        u32::from_le_bytes([values[12], values[13], values[14], values[15]]),
    ]
}

#[cfg(test)]
mod tests {
    use crate::Packer;

    #[test]
    fn pack_values() {
        let mut packer = Packer::new();
        for _ in 0..64 {
            packer.load("8874.85000000").unwrap();
            packer.load("8875.14000000").unwrap();
            packer.load("8874.99000000").unwrap();
            packer.load("8874.98000000").unwrap();
        }
        
        packer.print();
    }
}
