use crate::packet::builder::PacketBuilder;

use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
use num_enum::TryFromPrimitive;

use std::convert::TryFrom;

pub const BIT_PER_BLOCK: u8 = 14;

#[derive(TryFromPrimitive)]
#[repr(u16)]
pub enum Block {
    Air = 0x00,
    Grass = 0x20,
    Dirt = 0x30,
}
pub struct PrimaryBitMask;

pub struct HeightMaps {
    data: Vec<u8>,
}

impl HeightMaps {
    pub fn new() -> Self {
        let mut data = Vec::new();
        let mut height_map = BitArray::new(9, 256);
        for i in 0..256 {
            height_map.set(i, 16);
        }
        let long_tag = String::from("MOTION_BLOCKING");
        let long_tag = long_tag.as_bytes();
        // Compound
        data.push(10);
        data.push(0);
        data.push(0);
        // Long array tag
        data.push(12);
        data.extend((long_tag.len() as u16).to_be_bytes().iter());
        data.extend(long_tag);
        // array size
        data.extend(36_i32.to_be_bytes().iter());
        for d in height_map.data.iter() {
            data.extend(d.to_be_bytes().iter());
        }
        // End Compound
        data.push(0);

        Self {
            data,
        }
    }
}

impl Default for HeightMaps {
    fn default() -> Self {
        Self::new()
    }
}


pub struct BlockEntities;
pub struct Biomes {
    data: [i32; 1024],
}
impl Biomes {
    pub fn new() -> Biomes {
        Self { data: [21; 1024] }
    }
}
impl Default for Biomes {
    fn default() -> Self {
        Self::new()
    }
}
pub struct ChunkPacket {
    full_chunk: bool,
    primary_bit_mask: i32,
    height_maps: HeightMaps,
    biomes: Biomes,
    data: ChunkColumn,
    block_entities: BlockEntities,
}

impl ChunkPacket {
    pub fn new(data: ChunkColumn) -> ChunkPacket {
        Self {
            full_chunk: true,
            primary_bit_mask: 0x1,
            height_maps: HeightMaps::new(),
            biomes: Biomes::new(),
            data,
            block_entities: BlockEntities,
        }
    }

    pub fn build(self) -> Vec<u8> {
        let mut builder = PacketBuilder::new();
        builder.push_varint(0x22);
        builder.push_int(self.data.location.0);
        builder.push_int(self.data.location.1);
        builder.push_bool(self.full_chunk);
        builder.push_varint(self.primary_bit_mask);
        builder.push_vec_u8(&self.height_maps.data);
        builder.push_vec_i32(&self.biomes.data);
        let mut data_size = 0;
        for section in self.data.sections.iter() {
            data_size += section.data.data.len() * 8;
        }
        builder.push_varint(data_size as i32);
        for section in self.data.sections.iter() {
            builder.push_vec_u64(&section.data.data);
        }
        builder.push_varint(0);
        // builder.push entity
        builder.build()
    }
}

pub struct ChunkColumn {
    location: (i32, i32),
    sections: Vec<ChunkSection>,
}

impl ChunkColumn {
    pub fn new(location: (i32, i32)) -> Self {
        Self {
            location,
            sections: vec![grass_chunk_section()],
        }
    }
}

fn grass_chunk_section() -> ChunkSection {
    let mut section = ChunkSection::new();
    for x in 0..15 {
        for y in 0..15 {
            for z in 0..15 {
                section.set_block_at(x, y, z, Block::Grass);
            }
        }
    }
    section
}

pub struct ChunkSection {
    block_count: u16,
    bits_per_block: u8,
    data: BitArray,
}

impl ChunkSection {
    pub fn new() -> Self {
        ChunkSection {
            block_count: 0,
            bits_per_block: BIT_PER_BLOCK,
            data: BitArray::new(BIT_PER_BLOCK, 16 * 16 * 16),
        }
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> Result<Block> {
        let index = (y << 8) | (z << 4) | x;
        let block = self.data.get(index);
        Block::try_from(block as u16).wrap_err(format!("Block ID: {:x} is not supproted.", block))
    }

    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, block: Block) {
        let index = (y << 8) | (z << 4) | x;
        let old_block = self.block_at(x, y, z).unwrap();
        if let Block::Air = block {
            match old_block {
                Block::Air => (),
                _ => self.block_count -= 1,
            }
        } else if let Block::Air = old_block {
            self.block_count += 1;
        }
        self.data.set(index, block as u64);
    }
}

impl Default for ChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

// https://github.com/feather-rs/feather/blob/develop/core/chunk/src/lib.rs#L908
pub struct BitArray {
    data: Vec<u64>,
    capacity: usize,
    bits_per_value: u8,
    value_mask: u64,
}

impl BitArray {
    pub fn new(bits_per_value: u8, capacity: usize) -> Self {
        assert!(
            bits_per_value <= 64,
            "Bits per value cannot be more than 64"
        );
        assert!(bits_per_value > 0, "Bits per value must be positive");
        let data = {
            let len = (((capacity * (bits_per_value as usize)) as f64) / 64.0).ceil() as usize;
            vec![0u64; len]
        };

        let value_mask = (1 << (bits_per_value as u64)) - 1;

        Self {
            data,
            capacity,
            bits_per_value,
            value_mask,
        }
    }

    /// Creates a new `BitArray` based on the given raw parts.
    pub fn from_raw(data: Vec<u64>, bits_per_value: u8, capacity: usize) -> Self {
        assert!(
            bits_per_value <= 64,
            "Bits per value cannot be more than 64"
        );
        assert!(bits_per_value > 0, "Bits per value must be positive");

        let value_mask = (1 << (bits_per_value as u64)) - 1;

        Self {
            data,
            capacity,
            bits_per_value,
            value_mask,
        }
    }

    /// Returns the highest possible value represented
    /// by and entry in this `BitArray`.
    pub fn highest_possible_value(&self) -> u64 {
        self.value_mask
    }

    /// Returns the value at the given location in this `BitArray`.
    pub fn get(&self, index: usize) -> u64 {
        assert!(index < self.capacity, "Index out of bounds");

        let bit_index = index * (self.bits_per_value as usize);

        let start_long_index = bit_index / 64;

        let start_long = self.data[start_long_index];

        let index_in_start_long = (bit_index % 64) as u64;

        let mut result = start_long >> index_in_start_long;

        let end_bit_offset = index_in_start_long + self.bits_per_value as u64;

        if end_bit_offset > 64 {
            // Value stretches across multiple longs
            let end_long = self.data[start_long_index + 1];
            result |= end_long << (64 - index_in_start_long);
        }

        result & self.value_mask
    }

    /// Sets the value at the given index into this `BitArray`
    pub fn set(&mut self, index: usize, val: u64) {
        assert!(index < self.capacity, "Index out of bounds");
        assert!(
            val <= self.value_mask,
            "Value does not fit into bits_per_value"
        );

        let bit_index = index * (self.bits_per_value as usize);

        let start_long_index = bit_index / 64;

        let index_in_start_long = (bit_index % 64) as u64;

        // Clear bits of this value first
        self.data[start_long_index] = (self.data[start_long_index]
            & !(self.value_mask << index_in_start_long))
            | ((val & self.value_mask) << index_in_start_long);

        let end_bit_offset = index_in_start_long + self.bits_per_value as u64;
        if end_bit_offset > 64 {
            // Value stretches across multiple longs
            self.data[start_long_index + 1] = (self.data[start_long_index + 1]
                & !((1 << (end_bit_offset - 64)) - 1))
                | val >> (64 - index_in_start_long);
        }

        debug_assert_eq!(self.get(index), val);
    }

    /// Returns the internal array.
    pub fn inner(&self) -> &Vec<u64> {
        &self.data
    }
}
