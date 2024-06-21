use crate::bindings::exports::wasi;
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
        Ok(())
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
