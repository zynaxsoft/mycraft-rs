use async_std::{io::BufWriter, net::TcpListener, net::TcpStream, prelude::*, task};

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Section};

use mycraft::packet::{
    builder::PacketBuilder,
    codec::{Framed, McCodec},
    reader::McBytesReader,
    chunk::{ChunkPacket, ChunkColumn},
};

fn main() -> Result<(), Report> {
    color_eyre::install()?;
    task::block_on(accept_loop());
    Ok(())
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
        let report = dispatch(frame, &mut writer, &mut client).await;
        if report.is_err() {
            println!("{:?}", report);
        }
    }
    drop(stream);
}

async fn dispatch(
    data: Vec<u8>,
    writer: &mut BufWriter<&TcpStream>,
    client: &mut Client,
) -> Result<(), Report> {
    let mut reader = McBytesReader::from_vec(data);
    let packet_id = reader.read_varint()?;
    match client.state {
        ProtocolState::Handshaking => match packet_id {
            0x00 => {
                handshake(&mut reader).await?;
                client.state = ProtocolState::Login;
            }
            _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        ProtocolState::Login => match packet_id {
            0x00 => {
                login_start(&mut reader, writer).await?;
                client.state = ProtocolState::Play;
            }
            _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        ProtocolState::Play => match packet_id {
            0x00 => {
                println!("Teleport confirmed ID: {}", reader.read_varint()?);
            }
            0x11 => {
                let x = reader.read_double()?;
                let y = reader.read_double()?;
                let z = reader.read_double()?;
                let ground = reader.read_one_byte()?;
                println!("{:.2}, {:.2}, {:.2}, ground: {}",
                    x, y, z, ground);
            }
            0x0f => {
            }
            0x2a => {
                use std::time::Duration;
                task::sleep(Duration::from_millis(20)).await;
                if let ProtocolState::Play = client.state {
                    use std::time::SystemTime;
                    let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
                    let mut builder = PacketBuilder::new();
                    builder.push_varint(0x21);
                    builder.push_long(ts as i64 % 11121);
                    let buf = builder.build();
                    writer.write(buf.as_slice()).await?;
                    writer.flush().await?;
                    // println!("sent keep alive");
                    ticks(writer).await?;
                }
            }
            id => {
                println!("got {:#2x} on play", id);
            }
            // _ => println!("Got unsupported packet id: {:x}", packet_id),
        },
        _ => println!("Unsupported state {:?}", client.state),
    }
    Ok(())
}

async fn login_start(reader: &mut McBytesReader, writer: &mut BufWriter<&TcpStream>) -> Result<(), Report> {
    // see https://wiki.vg/Protocol_FAQ#What.27s_the_normal_login_sequence_for_a_client.3F
    let name = reader.read_string()?;
    println!("A user {} has requested login!", name);
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x02);
    builder.push_string("94ec47eb-5961-498b-be0d-25e1f9e4616b");
    builder.push_string("zynaxsoft");
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;

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
    writer.write(buf.as_slice()).await?;

    // Inventory
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x15);
    builder.push_byte(1);
    builder.push_short(0);
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;

    // Spawn Position
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x4e);
    builder.push_position(0, 0, 0);
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;

    // Chunk Data
    let chunk_column = ChunkColumn::new((0, 0));
    let chunk_packet = ChunkPacket::new(chunk_column);
    let buf = chunk_packet.build();
    writer.write(buf.as_slice()).await?;
    let chunk_column = ChunkColumn::new((1, 0));
    let chunk_packet = ChunkPacket::new(chunk_column);
    let buf = chunk_packet.build();
    writer.write(buf.as_slice()).await?;
    let chunk_column = ChunkColumn::new((0, 1));
    let chunk_packet = ChunkPacket::new(chunk_column);
    let buf = chunk_packet.build();
    writer.write(buf.as_slice()).await?;
    let chunk_column = ChunkColumn::new((1, 1));
    let chunk_packet = ChunkPacket::new(chunk_column);
    let buf = chunk_packet.build();
    writer.write(buf.as_slice()).await?;
    println!("sent chunk.");

    // Lighting
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x25);
    builder.push_varint(1);
    builder.push_varint(1);
    builder.push_varint(0b11_1111_1111_1111_1111);
    builder.push_varint(0b11_1111_1111_1111_1111);
    builder.push_varint(0b11_1111_1111_1111_1111);
    builder.push_varint(0b11_1111_1111_1111_1111);
    for _ in 0..18 {
        builder.push_varint(2048);
        builder.push_vec_u8(&[0xFF; 2048]);
    }
    for _ in 0..18 {
        builder.push_varint(2048);
        builder.push_vec_u8(&[0xFF; 2048]);
    }
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;

    use std::time::Duration;
    task::sleep(Duration::from_millis(200)).await;

    use std::time::SystemTime;
    let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();

    // Player Position And Look
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x36);
    builder.push_double(0.0);
    builder.push_double(64.0);
    builder.push_double(0.0);
    builder.push_float(0.0);
    builder.push_float(0.0);
    // builder.push_byte(0b0000_10101);
    builder.push_byte(0);
    builder.push_varint(ts as i32 % 237845);
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;
    println!("sent player position!");

    writer.flush().await?;

    Ok(())
}

static mut server_ticks: u64 = 0;

async fn ticks(writer: &mut BufWriter<&TcpStream>) -> Result<(), Report> {
    unsafe {
        server_ticks += 1;
    }
    let mut builder = PacketBuilder::new();
    builder.push_varint(0x4f);
    unsafe {builder.push_long(server_ticks as i64);}
    builder.push_long(6000);
    let buf = builder.build();
    writer.write(buf.as_slice()).await?;
    writer.flush().await?;
    Ok(())
}

async fn handshake(reader: &mut McBytesReader) -> Result<(), Report> {
    let protocol_version = reader.read_varint()?;
    let server_address = reader.read_string()?;
    let server_port = reader.read_unsigned_short()?;
    let next_state = reader.read_varint()?;
    println!(
        "{} {} {} {}",
        protocol_version, server_address, server_port, next_state
    );
    Ok(())
}
