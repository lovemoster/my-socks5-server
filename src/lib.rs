use std::{
    error::Error,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use tokio::{
    io::{self, split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

pub async fn handler<'a>(
    src_stream: TcpStream,
    _src_socket: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let (mut src_read, mut src_write) = split(src_stream);

    // 进行协议握手
    handshake(&mut src_read, &mut src_write).await?;

    // 握手完毕开始解析内容
    let req = parse_protocol(&mut src_read, &mut src_write).await?;

    let (mut dst_read, mut dst_write) = create_link(req, &mut src_read, &mut src_write).await?;

    tokio::spawn(async move {
        let _ = io::copy(&mut src_read, &mut dst_write).await;
    });

    let _ = io::copy(&mut dst_read, &mut src_write).await?;

    Ok(())
}

/// 进行协议握手
async fn handshake(
    src_read: &mut ReadHalf<TcpStream>,
    src_write: &mut WriteHalf<TcpStream>,
) -> Result<(), Box<dyn Error>> {
    let mut read_buffer = [0x00_u8; 1024];

    let _read_length = src_read.read(&mut read_buffer).await?;

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
    unsafe {
        static mut READ_BUFFER: [u8; 1024] = [0x00_u8; 1024];
        let read_length = src_read.read(&mut READ_BUFFER).await?;

        let atype = &READ_BUFFER[3..4];
        let dst_addr = &READ_BUFFER[4..read_length - 2];
        let dst_port = &READ_BUFFER[read_length - 2..read_length];

        let req: Request<'a> = Request {
            address_type: atype,
            address: dst_addr,
            port: dst_port,
        };

        Ok(req)
    }
}

async fn create_link<'a>(
    req: Request<'a>,
    _src_read: &'a mut ReadHalf<TcpStream>,
    src_write: &'a mut WriteHalf<TcpStream>,
) -> Result<(ReadHalf<TcpStream>, WriteHalf<TcpStream>), Box<dyn Error>> {
    // 根据不同的类型建立链接
    let mut atype = req.address_type;
    let address = req.address;
    let mut port_mut = req.port;
    let port = port_mut.read_u16().await?;

    let dst_stream = match atype.get(0) {
        Some(0x01_u8) => {
            let socket = SocketAddrV4::new(
                Ipv4Addr::new(address[0], address[1], address[2], address[3]),
                port,
            );
            TcpStream::connect(socket).await?
        }
        Some(0x03_u8) => {
            // let socket = SocketAddr::new(
            //     address,
            //     port,
            // );
            let address = &address[1..];
            let address = String::from_utf8_lossy(address).to_string();
            let host = format!("{}:{}", address, port);

            TcpStream::connect(&host).await?
        }
        Some(0x04_u8) => {
            let address_a = (&address[0..2]).read_u16().await?;
            let address_b = (&address[2..4]).read_u16().await?;
            let address_c = (&address[4..6]).read_u16().await?;
            let address_d = (&address[6..8]).read_u16().await?;
            let address_e = (&address[8..10]).read_u16().await?;
            let address_f = (&address[10..12]).read_u16().await?;
            let address_g = (&address[12..14]).read_u16().await?;
            let address_h = (&address[14..16]).read_u16().await?;

            let socket = SocketAddrV6::new(
                Ipv6Addr::new(
                    address_a, address_b, address_c, address_d, address_e, address_f, address_g,
                    address_h,
                ),
                port,
                0,
                0,
            );
            TcpStream::connect(socket).await?
        }
        Some(_n) => {
            return Err("sss".into());
        }
        None => {
            return Err("sss".into());
        }
    };

    let (dst_read, dst_write) = split(dst_stream);

    let lent = address.len() + 6;

    let mut buffer = Vec::with_capacity(lent);

    buffer.insert(0, 0x05);
    buffer.insert(1, 0x00);
    buffer.insert(2, 0x00);
    buffer.insert(3, atype.read_u8().await?);

    let mut i = 3;
    for ele in address {
        i = i + 1;
        buffer.insert(i, *ele)
    }

    for ele in port_mut {
        i = i + 1;
        buffer.insert(i, *ele);
    }

    src_write.write(&buffer).await?;
    src_write.flush().await?;

    Ok((dst_read, dst_write))
}

struct Request<'a> {
    address_type: &'a [u8],
    address: &'a [u8],
    port: &'a [u8],
}
