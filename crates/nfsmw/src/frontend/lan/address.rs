//! An address as a player types it, for direct connect (the browser's row and `net.connect`). No name lookup: a lookup can
//! block, and LAN play needs none.

use std::net::{IpAddr, SocketAddr};

/// The IP and the port, if one was given: `1.2.3.4`, `1.2.3.4:9760`, `[::1]:9760`, `::1` or `localhost[:port]`.
pub fn address_of(address: &str) -> Result<(IpAddr, Option<u16>), String> {
    let address = address.trim();
    let bad = || format!("{address:?} is not an address (an IP, optionally with :port)");
    if let Ok(ip) = address.parse::<IpAddr>() {
        return Ok((ip, None));
    }
    if let Ok(socket) = address.parse::<SocketAddr>() {
        return Ok((socket.ip(), Some(socket.port())));
    }
    let (host, port) = match address.rsplit_once(':') {
        Some((host, port)) => (host, Some(port.parse::<u16>().map_err(|_| bad())?)),
        None => (address, None),
    };
    if !host.eq_ignore_ascii_case("localhost") {
        return Err(bad());
    }
    Ok((IpAddr::from([127, 0, 0, 1]), port))
}

/// The socket address of `address`, with `default_port` when it names none.
pub fn socket_of(address: &str, default_port: u16) -> Result<SocketAddr, String> {
    let (ip, port) = address_of(address)?;
    Ok(SocketAddr::new(ip, port.unwrap_or(default_port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_with_and_without_ports() {
        assert_eq!(socket_of("10.0.0.2", 9750), Ok("10.0.0.2:9750".parse().unwrap()));
        assert_eq!(socket_of(" 10.0.0.2:9800 ", 9750), Ok("10.0.0.2:9800".parse().unwrap()));
        assert_eq!(socket_of("[::1]:9800", 9750), Ok("[::1]:9800".parse().unwrap()));
        assert_eq!(socket_of("LOCALHOST", 9750), Ok("127.0.0.1:9750".parse().unwrap()));
        assert!(socket_of("lan-box", 9750).is_err());
        assert!(socket_of("10.0.0.2:x", 9750).is_err());
    }
}
