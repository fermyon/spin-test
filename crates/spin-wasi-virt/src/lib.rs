// TODO: remove this when things are closer to being implemented
#![allow(warnings)]

#[allow(warnings)]
mod bindings;

use bindings::exports::wasi;

struct Component;

impl wasi::cli::environment::Guest for Component {
    fn get_environment() -> Vec<(String, String)> {
        // TODO: Implement this
        Vec::new()
    }

    fn get_arguments() -> Vec<String> {
        Vec::new()
    }

    fn initial_cwd() -> Option<String> {
        todo!()
    }
}

impl wasi::cli::exit::Guest for Component {
    fn exit(status: Result<(), ()>) {
        todo!()
    }
}

impl wasi::filesystem::preopens::Guest for Component {
    fn get_directories() -> Vec<(wasi::filesystem::preopens::Descriptor, String)> {
        Vec::new()
    }
}

impl wasi::filesystem::types::Guest for Component {
    type Descriptor = Descriptor;

    type DirectoryEntryStream = DirectoryEntryStream;

    fn filesystem_error_code(
        err: &wasi::filesystem::types::Error,
    ) -> Option<wasi::filesystem::types::ErrorCode> {
        todo!()
    }
}

struct Descriptor;

impl wasi::filesystem::types::GuestDescriptor for Descriptor {
    fn read_via_stream(
        &self,
        offset: wasi::filesystem::types::Filesize,
    ) -> Result<wasi::filesystem::types::InputStream, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn write_via_stream(
        &self,
        offset: wasi::filesystem::types::Filesize,
    ) -> Result<wasi::filesystem::types::OutputStream, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn append_via_stream(
        &self,
    ) -> Result<wasi::filesystem::types::OutputStream, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn advise(
        &self,
        offset: wasi::filesystem::types::Filesize,
        length: wasi::filesystem::types::Filesize,
        advice: wasi::filesystem::types::Advice,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn sync_data(&self) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn get_flags(
        &self,
    ) -> Result<wasi::filesystem::types::DescriptorFlags, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn get_type(
        &self,
    ) -> Result<wasi::filesystem::types::DescriptorType, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn set_size(
        &self,
        size: wasi::filesystem::types::Filesize,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn set_times(
        &self,
        data_access_timestamp: wasi::filesystem::types::NewTimestamp,
        data_modification_timestamp: wasi::filesystem::types::NewTimestamp,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn read(
        &self,
        length: wasi::filesystem::types::Filesize,
        offset: wasi::filesystem::types::Filesize,
    ) -> Result<(Vec<u8>, bool), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn write(
        &self,
        buffer: Vec<u8>,
        offset: wasi::filesystem::types::Filesize,
    ) -> Result<wasi::filesystem::types::Filesize, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn read_directory(
        &self,
    ) -> Result<wasi::filesystem::types::DirectoryEntryStream, wasi::filesystem::types::ErrorCode>
    {
        todo!()
    }

    fn sync(&self) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn create_directory_at(&self, path: String) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn stat(
        &self,
    ) -> Result<wasi::filesystem::types::DescriptorStat, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn stat_at(
        &self,
        path_flags: wasi::filesystem::types::PathFlags,
        path: String,
    ) -> Result<wasi::filesystem::types::DescriptorStat, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn set_times_at(
        &self,
        path_flags: wasi::filesystem::types::PathFlags,
        path: String,
        data_access_timestamp: wasi::filesystem::types::NewTimestamp,
        data_modification_timestamp: wasi::filesystem::types::NewTimestamp,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn link_at(
        &self,
        old_path_flags: wasi::filesystem::types::PathFlags,
        old_path: String,
        new_descriptor: wasi::filesystem::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn open_at(
        &self,
        path_flags: wasi::filesystem::types::PathFlags,
        path: String,
        open_flags: wasi::filesystem::types::OpenFlags,
        flags: wasi::filesystem::types::DescriptorFlags,
    ) -> Result<wasi::filesystem::types::Descriptor, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn readlink_at(&self, path: String) -> Result<String, wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn remove_directory_at(&self, path: String) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn rename_at(
        &self,
        old_path: String,
        new_descriptor: wasi::filesystem::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn symlink_at(
        &self,
        old_path: String,
        new_path: String,
    ) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn unlink_file_at(&self, path: String) -> Result<(), wasi::filesystem::types::ErrorCode> {
        todo!()
    }

    fn is_same_object(&self, other: wasi::filesystem::types::DescriptorBorrow<'_>) -> bool {
        todo!()
    }

    fn metadata_hash(
        &self,
    ) -> Result<wasi::filesystem::types::MetadataHashValue, wasi::filesystem::types::ErrorCode>
    {
        todo!()
    }

    fn metadata_hash_at(
        &self,
        path_flags: wasi::filesystem::types::PathFlags,
        path: String,
    ) -> Result<wasi::filesystem::types::MetadataHashValue, wasi::filesystem::types::ErrorCode>
    {
        todo!()
    }
}

struct DirectoryEntryStream;

impl wasi::filesystem::types::GuestDirectoryEntryStream for DirectoryEntryStream {
    fn read_directory_entry(
        &self,
    ) -> Result<Option<wasi::filesystem::types::DirectoryEntry>, wasi::filesystem::types::ErrorCode>
    {
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

struct ResolveAddressStream;

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

struct Network;

impl wasi::sockets::network::GuestNetwork for Network {}

impl wasi::sockets::tcp::Guest for Component {
    type TcpSocket = TcpSocket;
}

struct TcpSocket;

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

struct UdpSocket;

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

struct IncomingDatagramStream;

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

struct OutgoingDatagramStream;

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

bindings::export!(Component with_types_in bindings);
