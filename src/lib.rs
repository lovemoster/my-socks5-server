use std::{error::Error, net::SocketAddr, ops::Deref};

use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

pub async fn handler(src_stream: TcpStream, _src_socket: SocketAddr) -> Result<(), Box<dyn Error>> {
    let (mut src_read, mut src_write) = split(src_stream);

    // 进行协议握手
    handshake(&mut src_read, &mut src_write).await?;

    // 握手完毕开始解析内容
    let req = parse_protocol(&mut src_read, &mut src_write).await?;

    let address_type = req.address_type;
    let address = req.address;
    let port = req.port;

    Ok(())
}

/// 进行协议握手
async fn handshake(
    src_read: &mut ReadHalf<TcpStream>,
    src_write: &mut WriteHalf<TcpStream>,
) -> Result<(), Box<dyn Error>> {
    let mut read_buffer = [0x00_u8; 1024];

    let read_length = src_read.read(&mut read_buffer).await?;

    let mut write_buffer = [0x00_u8; 2];
    write_buffer[0] = 5;
    write_buffer[1] = 0x0;

    src_write.write(&mut write_buffer).await?;
    let _ = src_write.flush();

    Ok(())
}

async fn parse_protocol<'a>(
    src_read: &mut ReadHalf<TcpStream>,
    _src_write: &mut WriteHalf<TcpStream>,
) -> Result<Request<'a>, Box<dyn Error>> {
    let mut read_buffer: &'a [u8; 1024] = [0x00_u8; 1024];
    let read_length = src_read.read(&mut read_buffer).await?;

    let version = &read_buffer[0..1];
    let cmd = &read_buffer[1..2];
    let rsv = &read_buffer[2..3];
    let atype = &read_buffer[3..4];
    let dst_addr = &read_buffer[4..read_length - 2];
    let dst_port = &read_buffer[read_length - 2..read_length];

    let req: Request<'static> = Request {
        address_type: atype,
        address: dst_addr,
        port: dst_port,
    };

    Ok(req)
}

struct Request<'a> {
    address_type: &'a [u8],
    address: &'a [u8],
    port: &'a [u8],
}
