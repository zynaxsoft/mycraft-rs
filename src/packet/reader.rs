use std::vec::IntoIter;

#[derive(Debug, Clone, Copy)]
pub enum McBytesErr {
    VarintTooBig,
    InsufficientBytes,
}

/// Minecraft bytes
#[derive(Debug)]
pub struct McBytesReader {
    iter: IntoIter<u8>,
}

impl McBytesReader {
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            iter: data.into_iter(),
        }
    }

    pub fn read_varint(&mut self) -> Result<i32, McBytesErr> {
        let mut num_read: i32 = 0;
        let mut result: i32 = 0;
        let mut buf: u8;
        loop {
            buf = self.iter.next().unwrap();
            let value = buf & 0b01111111;
            result |= (value as i32) << (7 * num_read);
            num_read += 1;
            if num_read > 5 {
                return Err(McBytesErr::VarintTooBig);
            }
            if (buf & 0b10000000) == 0 {
                break;
            }
        }
        Ok(result)
    }

    pub fn read_unsigned_short(&mut self) -> Result<u16, McBytesErr> {
        let result: u16 = self.read_one_byte()? as u16 + ((self.read_one_byte()? as u16) << 8);
        Ok(result)
    }

    pub fn read_string(&mut self) -> Result<String, McBytesErr> {
        let length = self.read_varint()?;
        let result = self.read_bytes(length)?;
        Ok(String::from_utf8_lossy(result.as_slice()).into_owned())
    }

    pub fn read_one_byte(&mut self) -> Result<u8, McBytesErr> {
        self.iter.next().ok_or(McBytesErr::InsufficientBytes)
    }

    pub fn read_bytes(&mut self, n: i32) -> Result<Vec<u8>, McBytesErr> {
        let mut result = Vec::new();
        for _ in 0..n {
            result.push(self.read_one_byte()?);
        }
        Ok(result)
    }
}

