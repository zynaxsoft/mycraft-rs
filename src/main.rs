use async_std::{io::BufWriter, net::TcpListener, net::TcpStream, prelude::*, task};

use mycraft::packet::{
    builder::PacketBuilder,
    codec::{Framed, McCodec},
    reader::McBytesReader,
};

fn main() {
    task::block_on(accept_loop());
}

async fn accept_loop() {
    let listener = TcpListener::bind("0.0.0.0:7781").await.unwrap();
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream.unwrap();
        println!("Incoming!!!!");
        task::spawn(async move { client_loop(stream).await });
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProtocolState {
    Handshaking,
    Status,
    Login,
    Play,
}

struct Client {
    state: ProtocolState,
}

impl Client {
    pub fn new() -> Self {
        Self {
            state: ProtocolState::Handshaking,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

async fn client_loop(stream: TcpStream) {
    let mut framed = Framed::new(&stream, McCodec);
    let mut writer = BufWriter::new(&stream);
    let mut client = Client::new();
    while let Some(frame) = framed.next().await.transpose().unwrap() {
        dispatch(frame, &mut writer, &mut client).await;
    }
    drop(stream);
}

async fn dispatch(data: Vec<u8>, writer: &mut BufWriter<&TcpStream>, client: &mut Client) {
    let mut reader = McBytesReader::from_vec(data);
    let packet_id = reader.read_varint().unwrap();
    match client.state {
        ProtocolState::Handshaking => match packet_id {
            0x00 => {
                handshake(&mut reader).await;
                client.state = ProtocolState::Login;
            }
            _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        ProtocolState::Login => match packet_id {
            0x00 => {
                login_start(&mut reader, writer).await;
                client.state = ProtocolState::Play;
            }
            _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        ProtocolState::Play => match packet_id {
            id @ 0x00..=0xFF => {
                println!("got {} on play", id);
            }
            _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        _ => println!("Unsupported state {:?}", client.state),
    }
}

async fn login_start(reader: &mut McBytesReader, writer: &mut BufWriter<&TcpStream>) {
    // see https://wiki.vg/Protocol_FAQ#What.27s_the_normal_login_sequence_for_a_client.3F
    let name = reader.read_string().unwrap();
    println!("A user {} has requested login!", name);
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x02);
    builder.push_string("94ec47eb-5961-498b-be0d-25e1f9e4616b");
    builder.push_string("zynaxsoft");
    let buf = builder.build();
    writer.write(buf.as_slice()).await.unwrap();

    // Join Game
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x26);
    builder.push_int(100);
    builder.push_byte(0);
    builder.push_int(0);
    builder.push_long(1);
    builder.push_byte(1);
    builder.push_string("default");
    builder.push_varint(10);
    builder.push_bool(true);
    builder.push_bool(false);
    let buf = builder.build();
    writer.write(buf.as_slice()).await.unwrap();

    // Inventory
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x15);
    builder.push_byte(1);
    builder.push_short(0);
    let buf = builder.build();
    writer.write(buf.as_slice()).await.unwrap();

    // Spawn Position
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x4e);
    builder.push_position(0, 0, 0);
    let buf = builder.build();
    writer.write(buf.as_slice()).await.unwrap();

    // Chunks
    // let mut builder = PacketBuilder::new();
    // builder.push_varint(0x26);
    // let buf = builder.build();
    // writer.write(buf.as_slice()).await.unwrap();

    // Player Position And Look
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x36);
    builder.push_double(0.0);
    builder.push_double(0.0);
    builder.push_double(0.0);
    builder.push_float(0.0);
    builder.push_float(0.0);
    builder.push_byte(0);
    builder.push_varint(1);
    let buf = builder.build();
    writer.write(buf.as_slice()).await.unwrap();

    writer.flush().await.unwrap();
}

async fn handshake(reader: &mut McBytesReader) {
    let protocol_version = reader.read_varint().unwrap();
    let server_address = reader.read_string().unwrap();
    let server_port = reader.read_unsigned_short().unwrap();
    let next_state = reader.read_varint().unwrap();
    println!(
        "{} {} {} {}",
        protocol_version, server_address, server_port, next_state
    );
}
