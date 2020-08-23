#[derive(Debug)]
pub struct PacketBuilder {
    data: Vec<u8>,
}

impl Default for PacketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push_varint(&mut self, n: i32) {
        let mut x = n;
        loop {
            let mut temp = (x & 0b0111_1111) as u8;
            x >>= 7;
            if x != 0 {
                temp |= 0b1000_0000;
            }
            self.data.push(temp);
            if x == 0 {
                break;
            }
        }
    }

    pub fn push_position(&mut self, x: i64, y: i64, z: i64) {
        let temp = ((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF);
        for b in temp.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_unsigned_char(&mut self, n: u16) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_int(&mut self, n: i32) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_short(&mut self, n: i16) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_long(&mut self, n: i64) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_float(&mut self, n: f32) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_double(&mut self, n: f64) {
        for b in n.to_be_bytes().iter() {
            self.push_byte(*b);
        }
    }

    pub fn push_bool(&mut self, n: bool) {
        self.push_byte(n as u8);
    }

    pub fn push_byte(&mut self, n: u8) {
        self.data.push(n);
    }

    pub fn push_vec_u8(&mut self, v: &[u8]) {
        self.data.extend(v);
    }

    pub fn push_vec_u64(&mut self, v: &[u64]) {
        for long in v.iter() {
            self.data.extend(long.to_be_bytes().iter());
        }
    }

    pub fn push_vec_i32(&mut self, v: &[i32]) {
        for long in v.iter() {
            self.data.extend(long.to_be_bytes().iter());
        }
    }

    pub fn push_string(&mut self, string: &str) {
        let bytes = string.as_bytes();
        self.push_varint(bytes.len() as i32);
        self.data.extend(bytes.iter())
    }

    pub fn build(self) -> Vec<u8> {
        let mut result = Vec::new();
        let mut x = self.data.len();
        loop {
            let mut temp = (x & 0b0111_1111) as u8;
            x >>= 7;
            if x != 0 {
                temp |= 0b1000_0000;
            }
            result.push(temp);
            if x == 0 {
                break;
            }
        }
        result.extend(self.data);
        result
    }
}
