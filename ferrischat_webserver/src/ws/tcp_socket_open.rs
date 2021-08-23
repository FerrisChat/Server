//! this is cursed code
//! do not mention it to anyone

use libc::{
    __errno_location, c_int, connect, getprotobynumber, setsockopt, sockaddr, socket, socklen_t,
    AF_INET, AF_INET6, SOCK_STREAM, TCP_REPAIR,
};
use std::net::TcpStream;
use std::os::unix::io::{FromRawFd, RawFd};

macro_rules! handle_error {
    ($input:expr) => {{
        if $input == -1 {
            let last_err = __errno_location();
            return Err(if last_err.is_null() {
                -1
            } else {
                unsafe { *last_err }
            });
        }
    }};
}

/// Open a TCP connection to a host without doing the opening handshake.
///
/// # Safety
/// As this calls raw C code, there are many ways for it to go wrong.
/// (no actual safety doc here, deal with it)
///
/// # Returns
/// A value of Ok() contains the TCP stream itself. Note that no TCP handshake has been performed on this stream.
///
/// A value of Err() means something went wrong, and returns a `c_int` containing the returned code.
/// If this code is `-1`, the last error was a null pointer.
pub unsafe fn connect_to_host(address: *const sockaddr, ipv6: bool) -> Result<TcpStream, c_int> {
    // create socket endpoint
    let fd = unsafe {
        socket(
            if ipv6 { AF_INET6 } else { AF_INET },
            SOCK_STREAM,
            0 as c_int,
        )
    };
    handle_error!(fd);

    // get info about the TCP protocol
    let tcp_proto = unsafe { getprotobynumber(6) };
    if tcp_proto.is_null() {
        return Err(-1);
    }
    let tcp_proto = unsafe { *tcp_proto };

    // set the TCP_REPAIR flag
    let set_opt = unsafe {
        setsockopt(
            fd,
            tcp_proto.p_proto,
            TCP_REPAIR,
            1 as *const c_int,
            std::mem::size_of::<c_int>() as socklen_t,
        )
    };
    handle_error!(set_opt);

    // establish connection to the underlying socket (this does nothing because the TCP_REPAIR flag is set
    let conn = unsafe { connect(fd, address, std::mem::sizeof::<sockaddr>() as socklen_t) };
    handle_error!(conn);

    // unset the TCP_REPAIR flag
    let unset_opt = unsafe {
        setsockopt(
            fd,
            tcp_proto.p_proto,
            TCP_REPAIR,
            0 as *const c_int,
            std::mem::size_of::<c_int>() as socklen_t,
        )
    };
    handle_error!(unset_opt);

    Ok(unsafe { std::net::TcpStream::from_raw_fd(fd) })
}
