use core::hash;

use crate::bindings::exports::wasi::{self, sockets::network::Ipv4SocketAddress};
use crate::Component;

use super::io::{Buffer, InputStream, OutputStream};

impl wasi::sockets::tcp_create_socket::Guest for Component {
    fn create_tcp_socket(
        address_family: wasi::sockets::tcp_create_socket::IpAddressFamily,
    ) -> Result<
        wasi::sockets::tcp_create_socket::TcpSocket,
        wasi::sockets::tcp_create_socket::ErrorCode,
    > {
        Ok(wasi::sockets::tcp_create_socket::TcpSocket::new(TcpSocket))
    }
}

impl wasi::sockets::tcp::Guest for Component {
    type TcpSocket = TcpSocket;
}

pub struct TcpSocket;

impl wasi::sockets::tcp::GuestTcpSocket for TcpSocket {
    fn start_bind(
        &self,
        network: wasi::sockets::tcp::NetworkBorrow<'_>,
        local_address: wasi::sockets::tcp::IpSocketAddress,
    ) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn finish_bind(&self) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn start_connect(
        &self,
        network: wasi::sockets::tcp::NetworkBorrow<'_>,
        remote_address: wasi::sockets::tcp::IpSocketAddress,
    ) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        let allowed_hosts = crate::manifest::AppManifest::allowed_hosts()
            .map_err(|e| wasi::sockets::tcp::ErrorCode::PermanentResolverFailure)?;
        let configs = match allowed_hosts {
            // If all hosts are allowed, then we can skip the rest of the checks.
            spin_outbound_networking::AllowedHostsConfig::All => return Ok(()),
            spin_outbound_networking::AllowedHostsConfig::SpecificHosts(configs) => configs,
        };

        let (remote_address, remote_port) = match remote_address {
            wasi::sockets::network::IpSocketAddress::Ipv4(i) => (
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                    i.address.0,
                    i.address.1,
                    i.address.2,
                    i.address.3,
                )),
                i.port,
            ),
            wasi::sockets::network::IpSocketAddress::Ipv6(i) => (
                std::net::IpAddr::V6(std::net::Ipv6Addr::new(
                    i.address.0,
                    i.address.1,
                    i.address.2,
                    i.address.3,
                    i.address.4,
                    i.address.5,
                    i.address.6,
                    i.address.7,
                )),
                i.port,
            ),
        };

        for config in configs {
            // Check if the port is allowed.
            let mut allowed_port = false;
            match config.port() {
                spin_outbound_networking::PortConfig::Any => {
                    allowed_port = true;
                    break;
                }
                spin_outbound_networking::PortConfig::List(l) => {
                    for port in l {
                        match port {
                            spin_outbound_networking::IndividualPortConfig::Port(p)
                                if *p == remote_port =>
                            {
                                allowed_port = true;
                                break;
                            }
                            spin_outbound_networking::IndividualPortConfig::Range(r)
                                if r.contains(&remote_port) =>
                            {
                                allowed_port = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if !allowed_port {
                return Err(wasi::sockets::tcp::ErrorCode::AccessDenied);
            }

            // If the scheme isn't a `*`, then this config does not grant access.
            if !config.scheme().allows_any() {
                continue;
            }

            match config.host() {
                spin_outbound_networking::HostConfig::AnySubdomain(_)
                | spin_outbound_networking::HostConfig::ToSelf => continue,
                spin_outbound_networking::HostConfig::Any => return Ok(()),
                spin_outbound_networking::HostConfig::List(hosts) => {
                    // Check if any host is a CIDR block that contains the remote address.
                    for host in hosts {
                        // Parse the host as an `IpNet` cidr block and if it fails
                        // then try parsing again with `/32` appended to the end.
                        let Ok(ip_net) = host
                            .parse::<ipnet::IpNet>()
                            .or_else(|_| format!("{host}/32").parse())
                        else {
                            continue;
                        };
                        if ip_net.contains(&remote_address) {
                            return Ok(());
                        }
                    }
                }
                spin_outbound_networking::HostConfig::Cidr(ip_net) => {
                    // Check if the host is a CIDR block that contains the remote address.
                    if ip_net.contains(&remote_address) {
                        return Ok(());
                    }
                }
            }
        }

        Err(wasi::sockets::tcp::ErrorCode::AccessDenied)
    }

    fn finish_connect(
        &self,
    ) -> Result<
        (
            wasi::sockets::tcp::InputStream,
            wasi::sockets::tcp::OutputStream,
        ),
        wasi::sockets::tcp::ErrorCode,
    > {
        let shared = Buffer::empty();
        Ok((
            wasi::sockets::tcp::InputStream::new(InputStream::Buffered(shared.clone())),
            wasi::sockets::tcp::OutputStream::new(OutputStream::Buffered(shared)),
        ))
    }

    fn start_listen(&self) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn finish_listen(&self) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn accept(
        &self,
    ) -> Result<
        (
            wasi::sockets::tcp::TcpSocket,
            wasi::sockets::tcp::InputStream,
            wasi::sockets::tcp::OutputStream,
        ),
        wasi::sockets::tcp::ErrorCode,
    > {
        todo!()
    }

    fn local_address(
        &self,
    ) -> Result<wasi::sockets::tcp::IpSocketAddress, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn remote_address(
        &self,
    ) -> Result<wasi::sockets::tcp::IpSocketAddress, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn is_listening(&self) -> bool {
        todo!()
    }

    fn address_family(&self) -> wasi::sockets::tcp::IpAddressFamily {
        todo!()
    }

    fn set_listen_backlog_size(&self, value: u64) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn keep_alive_enabled(&self) -> Result<bool, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_keep_alive_enabled(&self, value: bool) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn keep_alive_idle_time(
        &self,
    ) -> Result<wasi::sockets::tcp::Duration, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_keep_alive_idle_time(
        &self,
        value: wasi::sockets::tcp::Duration,
    ) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn keep_alive_interval(
        &self,
    ) -> Result<wasi::sockets::tcp::Duration, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_keep_alive_interval(
        &self,
        value: wasi::sockets::tcp::Duration,
    ) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn keep_alive_count(&self) -> Result<u32, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_keep_alive_count(&self, value: u32) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn hop_limit(&self) -> Result<u8, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_hop_limit(&self, value: u8) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn receive_buffer_size(&self) -> Result<u64, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_receive_buffer_size(&self, value: u64) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn send_buffer_size(&self) -> Result<u64, wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn set_send_buffer_size(&self, value: u64) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }

    fn subscribe(&self) -> wasi::sockets::tcp::Pollable {
        todo!()
    }

    fn shutdown(
        &self,
        shutdown_type: wasi::sockets::tcp::ShutdownType,
    ) -> Result<(), wasi::sockets::tcp::ErrorCode> {
        todo!()
    }
}
