//! this is cursed code
//! do not mention it to anyone

use nix::{
    errno::Errno,
    sys::socket::{
        connect, setsockopt, socket, sockopt::TcpRepair, AddressFamily, InetAddr, SockAddr,
        SockFlag, SockProtocol, SockType,
    },
};
use std::net::TcpStream;
use std::os::unix::io::FromRawFd;

/// Open a TCP connection to a host without doing the opening handshake.
///
/// # Returns
/// A value of Ok() contains the TCP stream itself. Note that no TCP handshake has been performed on this stream.
///
/// A value of Err() contains a `nix::errno::Errno` with the returned error code, or `Errno::UnknownErrno` if a configuration error happened.
#[allow(unsafe_code)]
pub fn connect_to_host(address: &SockAddr) -> Result<TcpStream, Errno> {
    let ipv6 = match address {
        SockAddr::Inet(ref inet) => match inet {
            InetAddr::V4(_) => false,
            InetAddr::V6(_) => true,
        },
        _ => return Err(Errno::UnknownErrno),
    };
    // create socket endpoint
    let fd = socket(
        if ipv6 {
            AddressFamily::Inet6
        } else {
            AddressFamily::Inet
        },
        SockType::Stream,
        SockFlag::empty(),
        SockProtocol::Tcp,
    )?;

    // set the TCP_REPAIR flag
    setsockopt(fd, TcpRepair, &1)?;

    // establish connection to the underlying socket (this does nothing because the TCP_REPAIR flag is set)
    connect(fd, address)?;

    // unset the TCP_REPAIR flag
    setsockopt(fd, TcpRepair, &0)?;

    Ok(unsafe { TcpStream::from_raw_fd(fd) })
}
