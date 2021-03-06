use std::str::FromStr;

use rust_decimal::{Decimal,Error};
use bitpacking::{BitPacker8x, BitPacker};
use serde::{Serialize,Deserialize};

pub struct Packer {
    bitpacker: BitPacker8x,
    cache: Cache,
    packed: [Vec<Block>; 4],
    trim: bool,
}

impl Default for Packer {
    fn default() -> Self {
        Packer::new()
    }
}

#[derive(Default)]
struct Cache {
    buffer: Option<[u32; 4]>,
    head: [u32; 4],
    compressed: [Vec<u32>; 4],
}

#[derive(Serialize, Deserialize)]
struct Block {
    bits: u8,
    head: u32,
    vals: Vec<u8>,
}

impl Packer {
    pub fn new() -> Packer {
        Packer {
            bitpacker: BitPacker8x::new(),
            cache: Cache::default(),
            packed: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            trim: true,
        }
    }

    pub fn with_trim(&mut self, trim: bool) -> &mut Self {
        self.trim = trim;
        self
    }

    pub fn load(&mut self, value: &str) -> Result<(),Error> {
        let result = if self.trim { value.trim_matches('0') } else { value };
        self.load_decimal(Decimal::from_str(result)?);
        Ok(())
    }
    
    pub fn load_decimal(&mut self, value: Decimal) {
        let parsed = zip_u8(value.serialize());
        match self.cache.buffer {
            Some(last) => {
                self.cache.compressed[0].push(parsed[0] ^ last[0]);
                self.cache.compressed[1].push(parsed[1] ^ last[1]);
                self.cache.compressed[2].push(parsed[2] ^ last[2]);
                self.cache.compressed[3].push(parsed[3] ^ last[3]);
            },
            None => {
                self.cache.head[0] = parsed[0];
                self.cache.head[1] = parsed[1];
                self.cache.head[2] = parsed[2];
                self.cache.head[3] = parsed[3];
            }
        }
        self.cache.buffer = Some(parsed);

        if self.cache.compressed[0].len() == BitPacker8x::BLOCK_LEN {
            self.pack()
        }
    }

    fn pack(&mut self) {
        for i in 0..4 {
            let bits = self.bitpacker.num_bits(&self.cache.compressed[i]);
            let mut compressed = vec![0u8; (bits as usize) * BitPacker8x::BLOCK_LEN / 8];

            let _ = self.bitpacker.compress(&self.cache.compressed[i], &mut compressed[..], bits);

            self.packed[i].push(Block {
                bits,
                head: self.cache.head[i],
                vals: compressed.to_vec(),
            });
        }
        self.flush_cache()
    }

    fn flush_cache(&mut self) {
        self.cache = Cache::default()
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

fn unzip_u8(values: [u32; 4]) -> [u8; 16] {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use crate::{Block,Packer};

    #[test]
    fn pack_values() {
        let mut packer = Packer::new();
        for _ in 0..65 {
            packer.load("8874.85000000").unwrap();
            packer.load("8875.14000000").unwrap();
            packer.load("8874.99000000").unwrap();
            packer.load("8874.98000000").unwrap();
        }
        
        let expected = [[Block { bits: 0, head: 512, vals: [].to_vec() }], [Block { bits: 7, head: 887485, vals: [231, 243, 249, 124, 145, 72, 36, 18, 129, 64, 32, 16, 247, 251, 253, 126, 231, 243, 249, 124, 145, 72, 36, 18, 129, 64, 32, 16, 247, 251, 253, 126, 62, 159, 207, 231, 137, 68, 34, 145, 8, 4, 2, 129, 191, 223, 239, 247, 62, 159, 207, 231, 137, 68, 34, 145, 8, 4, 2, 129, 191, 223, 239, 247, 243, 249, 124, 62, 72, 36, 18, 137, 64, 32, 16, 8, 251, 253, 126, 191, 243, 249, 124, 62, 72, 36, 18, 137, 64, 32, 16, 8, 251, 253, 126, 191, 159, 207, 231, 243, 68, 34, 145, 72, 4, 2, 129, 64, 223, 239, 247, 251, 159, 207, 231, 243, 68, 34, 145, 72, 4, 2, 129, 64, 223, 239, 247, 251, 249, 124, 62, 159, 36, 18, 137, 68, 32, 16, 8, 4, 253, 126, 191, 223, 249, 124, 62, 159, 36, 18, 137, 68, 32, 16, 8, 4, 253, 126, 191, 223, 207, 231, 243, 249, 34, 145, 72, 36, 2, 129, 64, 32, 239, 247, 251, 253, 207, 231, 243, 249, 34, 145, 72, 36, 2, 129, 64, 32, 239, 247, 251, 253, 124, 62, 159, 207, 18, 137, 68, 34, 16, 8, 4, 2, 126, 191, 223, 239, 124, 62, 159, 207, 18, 137, 68, 34, 16, 8, 4, 2, 126, 191, 223, 239].to_vec() }], [Block { bits: 0, head: 0, vals: [].to_vec() }], [Block { bits: 0, head: 0, vals: [].to_vec() }]];
        for i in 0..4 {
            assert_eq!(packer.packed[i].len(), 1);
            assert_eq!(packer.packed[i].get(0).unwrap().bits, expected[i].get(0).unwrap().bits);
            assert_eq!(packer.packed[i].get(0).unwrap().head, expected[i].get(0).unwrap().head);
            assert_eq!(packer.packed[i].get(0).unwrap().vals, expected[i].get(0).unwrap().vals);
        }
    }
}
