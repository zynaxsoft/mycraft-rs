use async_codec::{Decode, DecodeResult, Encode, EncodeResult};

pub use async_codec::Framed;

pub struct McCodec;

impl Encode for McCodec {
    type Item = Vec<u8>;
    type Error = ();

    fn encode(&mut self, _item: &Vec<u8>, _buf: &mut [u8]) -> EncodeResult<()> {
        Ok(1).into()
    }
}

impl Decode for McCodec {
    type Item = Vec<u8>;
    type Error = ();

    fn decode(&mut self, buf: &mut [u8]) -> (usize, DecodeResult<Vec<u8>, ()>) {
        let (header_length, packet_length) = get_packet_length(buf);
        let total_length = header_length + packet_length as usize;
        if total_length > buf.len() {
            return (0, DecodeResult::UnexpectedEnd);
        }
        let result = Vec::from(&buf[header_length..]);
        (total_length, Ok(result).into())
    }
}

fn get_packet_length(header: &[u8]) -> (usize, i32) {
    let mut num_read: i32 = 0;
    let mut result: i32 = 0;
    let mut header = header.iter();
    loop {
        let x = header.next().unwrap();
        let value = x & 0b01111111;
        result |= (value as i32) << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            panic!("VarInt is too big");
        }
        if (x & 0b10000000) == 0 {
            break;
        }
    }
    (num_read as usize, result)
}
