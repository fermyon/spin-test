// TODO: remove this when things are closer to being implemented
#![allow(warnings)]

mod filesystem;
pub mod http;
pub mod http_helper;
pub mod io;

use crate::bindings::exports::wasi;
use crate::Component;

impl wasi::cli::stdout::Guest for Component {
    fn get_stdout() -> io::exports::streams::OutputStream {
        io::exports::streams::OutputStream::new(io::OutputStream::Host(
            crate::bindings::wasi::cli::stdout::get_stdout(),
        ))
    }
}

impl wasi::cli::stdin::Guest for Component {
    fn get_stdin() -> io::exports::streams::InputStream {
        io::exports::streams::InputStream::new(io::InputStream::Buffered(io::Buffer::empty()))
    }
}

impl wasi::cli::stderr::Guest for Component {
    fn get_stderr() -> wasi::cli::stderr::OutputStream {
        wasi::io::streams::OutputStream::new(io::OutputStream::Host(
            crate::bindings::wasi::cli::stderr::get_stderr(),
        ))
    }
}

impl wasi::cli::terminal_stdout::Guest for Component {
    fn get_terminal_stdout() -> Option<wasi::cli::terminal_stdout::TerminalOutput> {
        todo!()
    }
}

impl wasi::cli::terminal_stdin::Guest for Component {
    fn get_terminal_stdin() -> Option<wasi::cli::terminal_stdin::TerminalInput> {
        todo!()
    }
}

impl wasi::cli::terminal_stderr::Guest for Component {
    fn get_terminal_stderr() -> Option<wasi::cli::terminal_stderr::TerminalOutput> {
        todo!()
    }
}

impl wasi::cli::terminal_output::Guest for Component {
    type TerminalOutput = TerminalOutput;
}

pub struct TerminalOutput;

impl wasi::cli::terminal_output::GuestTerminalOutput for TerminalOutput {}

impl wasi::cli::terminal_input::Guest for Component {
    type TerminalInput = TerminalInput;
}

pub struct TerminalInput;

impl wasi::cli::terminal_input::GuestTerminalInput for TerminalInput {}

impl wasi::random::random::Guest for Component {
    fn get_random_bytes(len: u64) -> Vec<u8> {
        crate::bindings::wasi::random::random::get_random_bytes(len)
    }

    fn get_random_u64() -> u64 {
        crate::bindings::wasi::random::random::get_random_u64()
    }
}

impl wasi::random::insecure_seed::Guest for Component {
    fn insecure_seed() -> (u64, u64) {
        crate::bindings::wasi::random::insecure_seed::insecure_seed()
    }
}

impl wasi::random::insecure::Guest for Component {
    fn get_insecure_random_bytes(len: u64) -> Vec<u8> {
        crate::bindings::wasi::random::insecure::get_insecure_random_bytes(len)
    }

    fn get_insecure_random_u64() -> u64 {
        crate::bindings::wasi::random::insecure::get_insecure_random_u64()
    }
}

impl wasi::clocks::wall_clock::Guest for Component {
    fn now() -> wasi::clocks::wall_clock::Datetime {
        let now = crate::bindings::wasi::clocks::wall_clock::now();
        wasi::clocks::wall_clock::Datetime {
            seconds: now.seconds,
            nanoseconds: now.nanoseconds,
        }
    }

    fn resolution() -> wasi::clocks::wall_clock::Datetime {
        todo!()
    }
}

impl wasi::clocks::monotonic_clock::Guest for Component {
    fn now() -> wasi::clocks::monotonic_clock::Instant {
        crate::bindings::wasi::clocks::monotonic_clock::now()
    }

    fn resolution() -> wasi::clocks::monotonic_clock::Duration {
        todo!()
    }

    fn subscribe_instant(when: wasi::clocks::monotonic_clock::Instant) -> wasi::io::poll::Pollable {
        todo!()
    }

    fn subscribe_duration(
        when: wasi::clocks::monotonic_clock::Duration,
    ) -> wasi::io::poll::Pollable {
        todo!()
    }
}

impl wasi::cli::environment::Guest for Component {
    fn get_environment() -> Vec<(String, String)> {
        let Some(component) = crate::manifest::AppManifest::get_component() else {
            // If we don't have a component, we just accept the host environment
            return crate::bindings::wasi::cli::environment::get_environment();
        };

        component.environment.into_iter().collect()
    }

    fn get_arguments() -> Vec<String> {
        Vec::new()
    }

    fn initial_cwd() -> Option<String> {
        None
    }
}

impl wasi::cli::exit::Guest for Component {
    fn exit(status: Result<(), ()>) {
        todo!()
    }
}

impl wasi::sockets::instance_network::Guest for Component {
    fn instance_network() -> wasi::sockets::instance_network::Network {
        todo!()
    }
}

impl wasi::sockets::ip_name_lookup::Guest for Component {
    type ResolveAddressStream = ResolveAddressStream;

    fn resolve_addresses(
        network: wasi::sockets::ip_name_lookup::NetworkBorrow<'_>,
        name: String,
    ) -> Result<
        wasi::sockets::ip_name_lookup::ResolveAddressStream,
        wasi::sockets::ip_name_lookup::ErrorCode,
    > {
        todo!()
    }
}

pub struct ResolveAddressStream;

impl wasi::sockets::ip_name_lookup::GuestResolveAddressStream for ResolveAddressStream {
    fn resolve_next_address(
        &self,
    ) -> Result<
        Option<wasi::sockets::ip_name_lookup::IpAddress>,
        wasi::sockets::ip_name_lookup::ErrorCode,
    > {
        todo!()
    }

    fn subscribe(&self) -> wasi::sockets::ip_name_lookup::Pollable {
        todo!()
    }
}

impl wasi::sockets::network::Guest for Component {
    type Network = Network;
}

pub struct Network;

impl wasi::sockets::network::GuestNetwork for Network {}

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
        todo!()
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
        todo!()
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

impl wasi::sockets::tcp_create_socket::Guest for Component {
    fn create_tcp_socket(
        address_family: wasi::sockets::tcp_create_socket::IpAddressFamily,
    ) -> Result<
        wasi::sockets::tcp_create_socket::TcpSocket,
        wasi::sockets::tcp_create_socket::ErrorCode,
    > {
        todo!()
    }
}

impl wasi::sockets::udp::Guest for Component {
    type UdpSocket = UdpSocket;
    type IncomingDatagramStream = IncomingDatagramStream;
    type OutgoingDatagramStream = OutgoingDatagramStream;
}

pub struct UdpSocket;

impl wasi::sockets::udp::GuestUdpSocket for UdpSocket {
    fn start_bind(
        &self,
        network: wasi::sockets::udp::NetworkBorrow<'_>,
        local_address: wasi::sockets::udp::IpSocketAddress,
    ) -> Result<(), wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn finish_bind(&self) -> Result<(), wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn stream(
        &self,
        remote_address: Option<wasi::sockets::udp::IpSocketAddress>,
    ) -> Result<
        (
            wasi::sockets::udp::IncomingDatagramStream,
            wasi::sockets::udp::OutgoingDatagramStream,
        ),
        wasi::sockets::udp::ErrorCode,
    > {
        todo!()
    }

    fn local_address(
        &self,
    ) -> Result<wasi::sockets::udp::IpSocketAddress, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn remote_address(
        &self,
    ) -> Result<wasi::sockets::udp::IpSocketAddress, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn address_family(&self) -> wasi::sockets::udp::IpAddressFamily {
        todo!()
    }

    fn unicast_hop_limit(&self) -> Result<u8, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn set_unicast_hop_limit(&self, value: u8) -> Result<(), wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn receive_buffer_size(&self) -> Result<u64, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn set_receive_buffer_size(&self, value: u64) -> Result<(), wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn send_buffer_size(&self) -> Result<u64, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn set_send_buffer_size(&self, value: u64) -> Result<(), wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn subscribe(&self) -> wasi::sockets::udp::Pollable {
        todo!()
    }
}

pub struct IncomingDatagramStream;

impl wasi::sockets::udp::GuestIncomingDatagramStream for IncomingDatagramStream {
    fn receive(
        &self,
        max_results: u64,
    ) -> Result<Vec<wasi::sockets::udp::IncomingDatagram>, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn subscribe(&self) -> wasi::sockets::udp::Pollable {
        todo!()
    }
}

pub struct OutgoingDatagramStream;

impl wasi::sockets::udp::GuestOutgoingDatagramStream for OutgoingDatagramStream {
    fn check_send(&self) -> Result<u64, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn send(
        &self,
        datagrams: Vec<wasi::sockets::udp::OutgoingDatagram>,
    ) -> Result<u64, wasi::sockets::udp::ErrorCode> {
        todo!()
    }

    fn subscribe(&self) -> wasi::sockets::udp::Pollable {
        todo!()
    }
}

impl wasi::sockets::udp_create_socket::Guest for Component {
    fn create_udp_socket(
        address_family: wasi::sockets::udp_create_socket::IpAddressFamily,
    ) -> Result<
        wasi::sockets::udp_create_socket::UdpSocket,
        wasi::sockets::udp_create_socket::ErrorCode,
    > {
        todo!()
    }
}
