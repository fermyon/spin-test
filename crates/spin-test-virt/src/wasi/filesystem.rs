use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Mutex, OnceLock},
};

use spin_manifest::schema::v2::WasiFilesMount;

use crate::Component;

use super::io;
use crate::bindings::exports::wasi::filesystem as exports;

impl exports::preopens::Guest for Component {
    fn get_directories() -> Vec<(exports::preopens::Descriptor, String)> {
        vec![(
            exports::preopens::Descriptor::new(Descriptor::Directory),
            "/".to_owned(),
        )]
    }
}

impl exports::types::Guest for Component {
    type Descriptor = Descriptor;

    type DirectoryEntryStream = DirectoryEntryStream;

    fn filesystem_error_code(
        err: io::exports::error::ErrorBorrow<'_>,
    ) -> Option<exports::types::ErrorCode> {
        None
    }
}

pub enum Descriptor {
    Directory,
    File(Arc<Vec<u8>>),
}

impl exports::types::GuestDescriptor for Descriptor {
    fn read_via_stream(
        &self,
        offset: exports::types::Filesize,
    ) -> Result<exports::types::InputStream, exports::types::ErrorCode> {
        match self {
            Descriptor::Directory => todo!(),
            Descriptor::File(c) => Ok(exports::types::InputStream::new(io::InputStream::Buffered(
                Vec::clone(c).into(),
            ))),
        }
    }

    fn write_via_stream(
        &self,
        offset: exports::types::Filesize,
    ) -> Result<exports::types::OutputStream, exports::types::ErrorCode> {
        todo!()
    }

    fn append_via_stream(&self) -> Result<exports::types::OutputStream, exports::types::ErrorCode> {
        todo!()
    }

    fn advise(
        &self,
        offset: exports::types::Filesize,
        length: exports::types::Filesize,
        advice: exports::types::Advice,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn sync_data(&self) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn get_flags(&self) -> Result<exports::types::DescriptorFlags, exports::types::ErrorCode> {
        Ok(exports::types::DescriptorFlags::READ)
    }

    fn get_type(&self) -> Result<exports::types::DescriptorType, exports::types::ErrorCode> {
        Ok(match self {
            Descriptor::Directory => exports::types::DescriptorType::Directory,
            Descriptor::File(_) => exports::types::DescriptorType::RegularFile,
        })
    }

    fn set_size(&self, size: exports::types::Filesize) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn set_times(
        &self,
        data_access_timestamp: exports::types::NewTimestamp,
        data_modification_timestamp: exports::types::NewTimestamp,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn read(
        &self,
        length: exports::types::Filesize,
        offset: exports::types::Filesize,
    ) -> Result<(Vec<u8>, bool), exports::types::ErrorCode> {
        todo!()
    }

    fn write(
        &self,
        buffer: Vec<u8>,
        offset: exports::types::Filesize,
    ) -> Result<exports::types::Filesize, exports::types::ErrorCode> {
        todo!()
    }

    fn read_directory(
        &self,
    ) -> Result<exports::types::DirectoryEntryStream, exports::types::ErrorCode> {
        todo!()
    }

    fn sync(&self) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn create_directory_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn stat(&self) -> Result<exports::types::DescriptorStat, exports::types::ErrorCode> {
        Ok(exports::types::DescriptorStat {
            type_: match self {
                Descriptor::Directory => exports::types::DescriptorType::Directory,
                Descriptor::File(_) => exports::types::DescriptorType::RegularFile,
            },
            link_count: 0,
            size: 64,
            data_access_timestamp: None,
            data_modification_timestamp: None,
            status_change_timestamp: None,
        })
    }

    fn stat_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
    ) -> Result<exports::types::DescriptorStat, exports::types::ErrorCode> {
        crate::println!("stat_at: {:?}", path);
        Ok(exports::types::DescriptorStat {
            type_: exports::types::DescriptorType::RegularFile,
            link_count: 0,
            size: 64,
            data_access_timestamp: None,
            data_modification_timestamp: None,
            status_change_timestamp: None,
        })
    }

    fn set_times_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
        data_access_timestamp: exports::types::NewTimestamp,
        data_modification_timestamp: exports::types::NewTimestamp,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn link_at(
        &self,
        old_path_flags: exports::types::PathFlags,
        old_path: String,
        new_descriptor: exports::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn open_at(
        &self,
        _path_flags: exports::types::PathFlags,
        path: String,
        // TODO: respect open_flags and flags
        open_flags: exports::types::OpenFlags,
        flags: exports::types::DescriptorFlags,
    ) -> Result<exports::types::Descriptor, exports::types::ErrorCode> {
        let file = FileSystem::get(&path).ok_or_else(|| exports::types::ErrorCode::NoEntry)?;
        Ok(exports::types::Descriptor::new(Descriptor::File(file)))
    }

    fn readlink_at(&self, path: String) -> Result<String, exports::types::ErrorCode> {
        todo!()
    }

    fn remove_directory_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn rename_at(
        &self,
        old_path: String,
        new_descriptor: exports::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn symlink_at(
        &self,
        old_path: String,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn unlink_file_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }

    fn is_same_object(&self, other: exports::types::DescriptorBorrow<'_>) -> bool {
        todo!()
    }

    fn metadata_hash(
        &self,
    ) -> Result<exports::types::MetadataHashValue, exports::types::ErrorCode> {
        Ok(exports::types::MetadataHashValue { lower: 0, upper: 0 })
    }

    fn metadata_hash_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
    ) -> Result<exports::types::MetadataHashValue, exports::types::ErrorCode> {
        Ok(exports::types::MetadataHashValue { lower: 0, upper: 0 })
    }
}

pub struct DirectoryEntryStream;

impl exports::types::GuestDirectoryEntryStream for DirectoryEntryStream {
    fn read_directory_entry(
        &self,
    ) -> Result<Option<exports::types::DirectoryEntry>, exports::types::ErrorCode> {
        todo!()
    }
}

impl crate::bindings::exports::fermyon::spin_wasi_virt::fs_handler::Guest for Component {
    fn add_file(path: String, contents: Vec<u8>) {
        FileSystem::add(path, contents)
    }
}

struct FileSystem;
static FILES: OnceLock<Mutex<HashMap<String, Arc<Vec<u8>>>>> = OnceLock::new();

impl FileSystem {
    fn add(path: String, contents: Vec<u8>) {
        let mut files = Self::get_files();
        files.insert(path, Arc::new(contents));
    }

    fn get(path: &str) -> Option<Arc<Vec<u8>>> {
        let files = Self::get_files();
        files.get(path).cloned()
    }

    fn get_files() -> std::sync::MutexGuard<'static, HashMap<String, Arc<Vec<u8>>>> {
        let mut files = FILES
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        files
    }
}
