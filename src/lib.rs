use bitpacking::{BitPacker, BitPacker8x};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents `Decimals` in packed form.  (.0 = Compressed blocks, .1 = Count of decimals)
///
/// Usage example:
/// ```
/// use floatpack::{pack, unpack};
/// use rust_decimal_macros::*;
///
/// let values = vec![dec!(1.0), dec!(2.0), dec!(3.0)];
/// assert_eq!(values, unpack(&pack(&values[..])));
/// ```
///
/// How it works:
/// 1. The `Decimal` values are serialized in their components (4 x u32)
/// 2. The 4 component streams are individually compressed by storing their cumulative difference (XOR).
/// 3. The 4 compressed component streams are then bit-packed
pub type PackedDecimals = ([Vec<Block>; 4], usize);

struct Packer {
    bitpacker: BitPacker8x,
    cache: Cache,
    packed: PackedDecimals,
}

impl Default for Packer {
    fn default() -> Self {
        Self::new()
    }
}

struct Cache {
    buffer: Option<[u32; 4]>,
    head: [u32; 4],
    compressed: [[u32; BitPacker8x::BLOCK_LEN]; 4],
    idx: usize,
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            buffer: None,
            head: [0; 4],
            compressed: [[0; BitPacker8x::BLOCK_LEN]; 4],
            idx: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub bits: u8,
    pub head: u32,
    pub vals: Vec<u8>,
}

impl Packer {
    fn new() -> Packer {
        Packer {
            bitpacker: BitPacker8x::new(),
            cache: Cache::default(),
            packed: ([Vec::new(), Vec::new(), Vec::new(), Vec::new()], 0),
        }
    }

    fn load_decimal(&mut self, value: &Decimal) {
        let parsed = zip_u8(value.serialize());
        match self.cache.buffer {
            Some(last) => {
                for i in 0..4 {
                    self.cache.compressed[i][self.cache.idx] = parsed[i] ^ last[i];
                }
                self.cache.idx += 1;
            }
            None => self.cache.head = parsed,
        }
        self.cache.buffer = Some(parsed);

        if self.cache.idx == BitPacker8x::BLOCK_LEN {
            self.pack()
        }
    }

    fn pack(&mut self) {
        if self.cache.buffer.is_none() {
            return;
        }
        for i in 0..4 {
            let bits = self.bitpacker.num_bits(&self.cache.compressed[i]);
            let mut compressed = vec![0u8; (bits as usize) * BitPacker8x::BLOCK_LEN / 8];

            let _ = self
                .bitpacker
                .compress(&self.cache.compressed[i], &mut compressed[..], bits);

            self.packed.0[i].push(Block {
                bits,
                head: self.cache.head[i],
                vals: compressed,
            });
        }
        self.packed.1 += self.cache.idx + 1;
        self.cache = Cache::default();
    }
}

/// Pack and compress Decimals.
pub fn pack(values: &[Decimal]) -> PackedDecimals {
    let mut p = Packer::new();
    for d in values {
        p.load_decimal(d);
    }
    p.pack();
    p.packed
}

/// Unpack and decompress Decimals.
pub fn unpack(values: &PackedDecimals) -> Vec<Decimal> {
    let bitpacker = BitPacker8x::new();
    let buf = Vec::with_capacity(values.1);
    let mut unpacked = [buf.clone(), buf.clone(), buf.clone(), buf];
    for (i, blocks) in values.0.iter().enumerate() {
        for block in blocks {
            let mut decompress = [0u32; BitPacker8x::BLOCK_LEN];
            bitpacker.decompress(&block.vals, &mut decompress, block.bits);
            let mut last = block.head;
            unpacked[i].push(last);
            for v in decompress {
                last ^= v;
                unpacked[i].push(last)
            }
        }
    }
    let mut result = Vec::with_capacity(values.1);
    for i in 0..values.1 {
        let v = Decimal::deserialize(unzip_u8([
            unpacked[0][i],
            unpacked[1][i],
            unpacked[2][i],
            unpacked[3][i],
        ]));
        result.push(v);
    }
    result
}

fn zip_u8(values: [u8; 16]) -> [u32; 4] {
    [
        u32::from_le_bytes([values[0], values[1], values[2], values[3]]),
        u32::from_le_bytes([values[4], values[5], values[6], values[7]]),
        u32::from_le_bytes([values[8], values[9], values[10], values[11]]),
        u32::from_le_bytes([values[12], values[13], values[14], values[15]]),
    ]
}

fn unzip_u8(values: [u32; 4]) -> [u8; 16] {
    let v0 = values[0].to_le_bytes();
    let v1 = values[1].to_le_bytes();
    let v2 = values[2].to_le_bytes();
    let v3 = values[3].to_le_bytes();
    [
        v0[0], v0[1], v0[2], v0[3], v1[0], v1[1], v1[2], v1[3], v2[0], v2[1], v2[2], v2[3], v3[0],
        v3[1], v3[2], v3[3],
    ]
}

#[cfg(test)]
mod tests {
    use crate::{pack, unpack};
    use rust_decimal::prelude::*;
    use rust_decimal_macros::*;

    fn test_packing(values: &Vec<Decimal>) {
        assert_eq!(values, &unpack(&pack(&values[..])));
    }

    #[test]
    fn some_values() {
        test_packing(&vec![
            dec!(0.866089137820393),
            dec!(11.866089137820393),
            Decimal::ZERO,
            Decimal::ONE,
            Decimal::NEGATIVE_ONE,
            Decimal::MAX,
            Decimal::MIN,
            Decimal::ONE_HUNDRED,
            Decimal::ONE_THOUSAND,
            Decimal::TWO,
            dec!(-111.866089137820393),
        ])
    }

    #[test]
    fn random_values() {
        let mut values = Vec::new();
        for _ in 0..257 {
            let mut v: f64 = rand::random();
            if rand::random() {
                v -= 1.;
            }
            let d = Decimal::from_f64(v).unwrap();
            values.push(d);
        }
        test_packing(&values)
    }

    #[test]
    fn compression() {
        let mut values = Vec::with_capacity(1000);
        for v in 0..1000 {
            values.push(Decimal::from(v))
        }
        let v_des = bincode::serialize(&values).unwrap();
        let p_des = bincode::serialize(&pack(&values)).unwrap();
        assert!(v_des.len() > p_des.len() * 3);
    }
}
