use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use spin_manifest::schema::v2::WasiFilesMount;

use crate::Component;

use super::io;
use crate::bindings::exports::wasi::filesystem as exports;

impl exports::preopens::Guest for Component {
    fn get_directories() -> Vec<(exports::preopens::Descriptor, String)> {
        vec![(
            exports::preopens::Descriptor::new(Descriptor::Directory(Directory {
                path: "/".into(),
            })),
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

#[derive(Debug)]
pub enum Descriptor {
    Directory(Directory),
    File(Arc<Vec<u8>>),
}

impl Descriptor {
    fn len(&self) -> u64 {
        match self {
            Descriptor::Directory(_) => 0,
            Descriptor::File(c) => c.len() as u64,
        }
    }

    fn typ(&self) -> exports::types::DescriptorType {
        match self {
            Descriptor::Directory(_) => exports::types::DescriptorType::Directory,
            Descriptor::File(_) => exports::types::DescriptorType::RegularFile,
        }
    }

    fn get_stat(&self) -> exports::types::DescriptorStat {
        exports::types::DescriptorStat {
            type_: self.typ(),
            link_count: 0,
            size: self.len(),
            data_access_timestamp: None,
            data_modification_timestamp: None,
            status_change_timestamp: None,
        }
    }
}

#[derive(Debug)]
struct Directory {
    path: PathBuf,
}

impl Directory {
    /// Join a relative path to the directory's path
    fn join(&self, path: String) -> PathBuf {
        self.path.join(path)
    }
}

impl exports::types::GuestDescriptor for Descriptor {
    fn read_via_stream(
        &self,
        offset: exports::types::Filesize,
    ) -> Result<exports::types::InputStream, exports::types::ErrorCode> {
        match self {
            Descriptor::Directory(_) => todo!(),
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
        Err(exports::types::ErrorCode::Access)
    }

    fn get_flags(&self) -> Result<exports::types::DescriptorFlags, exports::types::ErrorCode> {
        Ok(exports::types::DescriptorFlags::READ)
    }

    fn get_type(&self) -> Result<exports::types::DescriptorType, exports::types::ErrorCode> {
        Ok(self.typ())
    }

    fn set_size(&self, size: exports::types::Filesize) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn set_times(
        &self,
        data_access_timestamp: exports::types::NewTimestamp,
        data_modification_timestamp: exports::types::NewTimestamp,
    ) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn read(
        &self,
        length: exports::types::Filesize,
        offset: exports::types::Filesize,
    ) -> Result<(Vec<u8>, bool), exports::types::ErrorCode> {
        use io::exports::streams::GuestInputStream;
        match self
            .read_via_stream(offset)?
            .get::<io::InputStream>()
            .read(length)
        {
            Ok(bytes) => Ok((bytes, false)),
            Err(io::exports::streams::StreamError::Closed) => Ok((Vec::new(), true)),
            Err(io::exports::streams::StreamError::LastOperationFailed(_)) => {
                Err(exports::types::ErrorCode::Io)
            }
        }
    }

    fn write(
        &self,
        buffer: Vec<u8>,
        offset: exports::types::Filesize,
    ) -> Result<exports::types::Filesize, exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn read_directory(
        &self,
    ) -> Result<exports::types::DirectoryEntryStream, exports::types::ErrorCode> {
        if self.get_type()? != exports::types::DescriptorType::Directory {
            return Err(exports::types::ErrorCode::NotDirectory);
        }
        todo!()
    }

    fn sync(&self) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn create_directory_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn stat(&self) -> Result<exports::types::DescriptorStat, exports::types::ErrorCode> {
        Ok(self.get_stat())
    }

    fn stat_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
    ) -> Result<exports::types::DescriptorStat, exports::types::ErrorCode> {
        match self {
            Descriptor::Directory(d) if path == "." => self.stat(),
            Descriptor::Directory(d) => {
                let path = d.join(path);
                let file = Descriptor::File(
                    FileSystem::get(path.to_str().unwrap())
                        .ok_or_else(|| exports::types::ErrorCode::NoEntry)?,
                );
                Ok(file.get_stat())
            }
            Descriptor::File(_) => self.stat(),
        }
    }

    fn set_times_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
        data_access_timestamp: exports::types::NewTimestamp,
        data_modification_timestamp: exports::types::NewTimestamp,
    ) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn link_at(
        &self,
        old_path_flags: exports::types::PathFlags,
        old_path: String,
        new_descriptor: exports::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn open_at(
        &self,
        _path_flags: exports::types::PathFlags,
        path: String,
        // TODO: respect open_flags and flags
        open_flags: exports::types::OpenFlags,
        flags: exports::types::DescriptorFlags,
    ) -> Result<exports::types::Descriptor, exports::types::ErrorCode> {
        let path = match self {
            Descriptor::Directory(d) => d.path.join(path),
            Descriptor::File(_) => todo!(),
        };
        let file = FileSystem::get(&path.to_str().unwrap())
            .ok_or_else(|| exports::types::ErrorCode::NoEntry)?;
        Ok(exports::types::Descriptor::new(Descriptor::File(file)))
    }

    fn readlink_at(&self, path: String) -> Result<String, exports::types::ErrorCode> {
        todo!()
    }

    fn remove_directory_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn rename_at(
        &self,
        old_path: String,
        new_descriptor: exports::types::DescriptorBorrow<'_>,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn symlink_at(
        &self,
        old_path: String,
        new_path: String,
    ) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn unlink_file_at(&self, path: String) -> Result<(), exports::types::ErrorCode> {
        Err(exports::types::ErrorCode::Access)
    }

    fn is_same_object(&self, other: exports::types::DescriptorBorrow<'_>) -> bool {
        todo!()
    }

    fn metadata_hash(
        &self,
    ) -> Result<exports::types::MetadataHashValue, exports::types::ErrorCode> {
        // TODO(rylev): Implement metadata hash calculation
        Ok(exports::types::MetadataHashValue { lower: 0, upper: 0 })
    }

    fn metadata_hash_at(
        &self,
        path_flags: exports::types::PathFlags,
        path: String,
    ) -> Result<exports::types::MetadataHashValue, exports::types::ErrorCode> {
        // TODO(rylev): Implement metadata hash calculation
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
        static FILES: OnceLock<Mutex<HashMap<String, Arc<Vec<u8>>>>> = OnceLock::new();
        FILES
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap()
    }
}
