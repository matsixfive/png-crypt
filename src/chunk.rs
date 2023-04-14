use std::fmt;

use crate::chunk_type::ChunkType;
use crate::{Error, Result};
use crc::{Crc, CRC_32_ISO_HDLC};

#[allow(unused)]
pub struct Chunk {
    length: u32,
    chunk_type: ChunkType,
    data:Vec<u8>,
    crc: u32,
}

#[allow(unused)]
impl Chunk {
    /// Creates a new `Chunk` with the given `ChunkType` and data.
    pub fn new(chunk_type: ChunkType, data: Vec<u8>) -> Self {
        let length = data.len() as u32;

        let mut chunk = Chunk {
            length,
            chunk_type,
            data,
            crc: 0xFFFFFFFF,
        };

        chunk.crc = chunk.crc();
        chunk
    }

    /// The length of the data portion of this chunk.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// The `ChunkType` of this chunk
    pub fn chunk_type(&self) -> &ChunkType {
        &self.chunk_type
    }

    /// The raw data contained in this chunk in bytes
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The CRC of this chunk
    pub fn crc(&self) -> u32 {
        pub const PNG_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        let mut digest = PNG_CRC.digest();
        digest.update(&self.chunk_type.chunk);
        digest.update(&self.data);
        digest.finalize()
    }

    /// Returns the data stored in this chunk as a `String`. This function will return an error
    /// if the stored data is not valid UTF-8.
    pub fn data_as_string(&self) -> Result<String> {
        match std::str::from_utf8(self.data()) {
            Ok(v) => Ok(v.to_string()),
            Err(e) => Err(Error::from("Invalid UTF-8 sequence")),
        }
    }

    /// Returns this chunk as a byte sequences described by the PNG spec.
    /// The following data is included in this byte sequence in order:
    /// 1) Length of the data *(4 bytes)*
    /// 2) Chunk type *(4 bytes)*
    /// 3) The data itself *(`length` bytes)*
    /// 4) The CRC of the chunk type and data *(4 bytes)*
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let length_bytes = self.length().to_be_bytes();
        bytes.extend_from_slice(&length_bytes);

        let chunk_type_bytes = self.chunk_type().chunk;
        bytes.extend_from_slice(&chunk_type_bytes);

        bytes.extend_from_slice(&self.data);

        let crc_bytes = self.crc().to_be_bytes();
        bytes.extend_from_slice(&crc_bytes);

        bytes
    }
}

impl TryFrom<&[u8]> for Chunk {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let length = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let chunk_type = ChunkType::try_from([bytes[4], bytes[5], bytes[6], bytes[7]])?;
        let data = bytes[8..(8 + length as usize)].to_vec();
        let crc = u32::from_be_bytes([
            bytes[8 + length as usize],
            bytes[9 + length as usize],
            bytes[10 + length as usize],
            bytes[11 + length as usize]
        ]);

        // Validate the CRC
        pub const PNG_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        let mut digest = PNG_CRC.digest();
        digest.update(&chunk_type.chunk);
        digest.update(&data);
        if digest.finalize() != crc {
            return Err(Error::from("Invalid CRC"));
        }

        Ok(Self {
            length,
            chunk_type,
            data,
            crc,
        })
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Chunk {{",)?;
        writeln!(f, "  Length: {}", self.length())?;
        writeln!(f, "  Type: {}", self.chunk_type())?;
        writeln!(f, "  Data: {} bytes", self.data().len())?;
        writeln!(f, "  Crc: {}", self.crc())?;
        writeln!(f, "}}",)?;
        Ok(())
    }
}

// #![allow(unused_variables)]
// fn main() {}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_type::ChunkType;
    use std::str::FromStr;

    fn testing_chunk() -> Chunk {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        Chunk::try_from(chunk_data.as_ref()).unwrap()
    }

    #[test]
    fn test_new_chunk() {
        let chunk_type = ChunkType::from_str("RuSt").unwrap();
        let data = "This is where your secret message will be!".as_bytes().to_vec();
        let chunk = Chunk::new(chunk_type, data);
        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_chunk_length() {
        let chunk = testing_chunk();
        assert_eq!(chunk.length(), 42);
    }

    #[test]
    fn test_chunk_type() {
        let chunk = testing_chunk();
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
    }

    #[test]
    fn test_chunk_string() {
        let chunk = testing_chunk();
        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");
        assert_eq!(chunk_string, expected_chunk_string);
    }

    #[test]
    fn test_chunk_crc() {
        let chunk = testing_chunk();
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_valid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref()).unwrap();

        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");

        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
        assert_eq!(chunk_string, expected_chunk_string);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_invalid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656333;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref());

        assert!(chunk.is_err());
    }

    #[test]
    pub fn test_chunk_trait_impls() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk: Chunk = TryFrom::try_from(chunk_data.as_ref()).unwrap();

        let _chunk_string = format!("{}", chunk);
    }
}

