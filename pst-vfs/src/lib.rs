#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use libpst::table::{ParallelTable, TOMBSTONE};
use libpst::offset::OffsetTable;

const MAX_NAME: usize = 128;
const MAX_DATA: usize = 4096;

// File type markers
pub const TYPE_FILE: u8 = b'F';
pub const TYPE_DIR: u8  = b'D';
pub const TYPE_LINK: u8 = b'L';

// Permission bits (packed into one byte)
pub const PERM_READ: u8    = 0b001;
pub const PERM_WRITE: u8   = 0b010;
pub const PERM_EXECUTE: u8 = 0b100;
pub const PERM_RW: u8      = PERM_READ | PERM_WRITE;
pub const PERM_RWX: u8     = PERM_READ | PERM_WRITE | PERM_EXECUTE;

// Column indices into the metadata table
const COL_TYPE: usize   = 0;
const COL_PERM: usize   = 1;
const COL_OWNER: usize  = 2;
const COL_FLAGS: usize  = 3;

#[derive(Debug)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    DirectoryNotEmpty,
    NameTooLong,
    DataTooLarge,
    PermissionDenied,
}

pub struct FileSystem {
    meta: ParallelTable,
    offsets: OffsetTable,
    names: Vec<Option<String>>,
    data: Vec<Option<Vec<u8>>>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            meta: ParallelTable::new(&["type", "perm", "owner", "flags"]),
            offsets: OffsetTable::new(),
            names: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn create_file(
        &mut self,
        path: &str,
        owner: u8,
        perm: u8,
    ) -> Result<usize, FsError> {
        if path.len() > MAX_NAME { return Err(FsError::NameTooLong); }
        if self.find_path(path).is_some() { return Err(FsError::AlreadyExists); }

        let physical = self.meta.append(&[TYPE_FILE, perm, owner, 0]);
        let logical = self.offsets.assign(physical);
        self.ensure_capacity(logical);
        self.names[logical] = Some(String::from(path));
        self.data[logical] = Some(Vec::new());
        Ok(logical)
    }

    pub fn create_dir(
        &mut self,
        path: &str,
        owner: u8,
        perm: u8,
    ) -> Result<usize, FsError> {
        if path.len() > MAX_NAME { return Err(FsError::NameTooLong); }
        if self.find_path(path).is_some() { return Err(FsError::AlreadyExists); }

        let physical = self.meta.append(&[TYPE_DIR, perm, owner, 0]);
        let logical = self.offsets.assign(physical);
        self.ensure_capacity(logical);
        self.names[logical] = Some(String::from(path));
        self.data[logical] = None;
        Ok(logical)
    }

    pub fn write(&mut self, logical_id: usize, content: &[u8]) -> Result<(), FsError> {
        if content.len() > MAX_DATA { return Err(FsError::DataTooLarge); }
        if !self.offsets.is_valid(logical_id) { return Err(FsError::NotFound); }
        if let Some(phys) = self.offsets.resolve(logical_id) {
            if self.meta.get(COL_TYPE, phys) == Some(TYPE_DIR) {
                return Err(FsError::NotADirectory);
            }
        }
        self.data[logical_id] = Some(content.to_vec());
        Ok(())
    }

    pub fn read(&self, logical_id: usize) -> Result<&[u8], FsError> {
        if !self.offsets.is_valid(logical_id) { return Err(FsError::NotFound); }
        match &self.data[logical_id] {
            Some(d) => Ok(d.as_slice()),
            None => Ok(&[]),
        }
    }

    pub fn delete(&mut self, logical_id: usize) -> Result<(), FsError> {
        if !self.offsets.is_valid(logical_id) { return Err(FsError::NotFound); }

        // If directory, check it's empty
        if let Some(phys) = self.offsets.resolve(logical_id) {
            if self.meta.get(COL_TYPE, phys) == Some(TYPE_DIR) {
                if let Some(name) = &self.names[logical_id] {
                    let prefix = if name.ends_with('/') {
                        name.clone()
                    } else {
                        let mut p = name.clone();
                        p.push('/');
                        p
                    };
                    if !self.ls(&prefix).is_empty() {
                        return Err(FsError::DirectoryNotEmpty);
                    }
                }
            }
        }

        if let Some(phys) = self.offsets.resolve(logical_id) {
            self.meta.tombstone(phys);
            self.offsets.invalidate(logical_id);
            self.names[logical_id] = None;
            self.data[logical_id] = None;
        }
        Ok(())
    }

    pub fn rename(&mut self, logical_id: usize, new_path: &str) -> Result<(), FsError> {
        if new_path.len() > MAX_NAME { return Err(FsError::NameTooLong); }
        if !self.offsets.is_valid(logical_id) { return Err(FsError::NotFound); }
        if self.find_path(new_path).is_some() { return Err(FsError::AlreadyExists); }
        self.names[logical_id] = Some(String::from(new_path));
        Ok(())
    }

    /// ls — scan for entries whose path starts with prefix.
    /// This IS the directory listing. No tree walk, just grep.
    pub fn ls(&self, prefix: &str) -> Vec<(usize, &str)> {
        let mut results = Vec::new();
        for (logical, name_opt) in self.names.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(name) = name_opt {
                if name.starts_with(prefix) {
                    // Only direct children — no deeper slashes after prefix
                    let rest = &name[prefix.len()..];
                    if !rest.contains('/') {
                        results.push((logical, name.as_str()));
                    }
                }
            }
        }
        results
    }

    /// find — scan for entries whose path contains the pattern anywhere.
    pub fn find(&self, pattern: &str) -> Vec<(usize, &str)> {
        let mut results = Vec::new();
        for (logical, name_opt) in self.names.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(name) = name_opt {
                if name.contains(pattern) {
                    results.push((logical, name.as_str()));
                }
            }
        }
        results
    }

    pub fn find_path(&self, path: &str) -> Option<usize> {
        for (logical, name_opt) in self.names.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if name_opt.as_deref() == Some(path) {
                return Some(logical);
            }
        }
        None
    }

    pub fn get_type(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.meta.get(COL_TYPE, phys)
    }

    pub fn get_name(&self, logical_id: usize) -> Option<&str> {
        self.names.get(logical_id)?.as_deref()
    }

    pub fn get_perm(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.meta.get(COL_PERM, phys)
    }

    pub fn compact(&mut self) {
        let remap = self.meta.compact();
        self.offsets.rebuild_from_remap(&remap);
    }

    pub fn file_count(&self) -> usize {
        self.meta.live_count()
    }

    fn ensure_capacity(&mut self, logical: usize) {
        while self.names.len() <= logical {
            self.names.push(None);
        }
        while self.data.len() <= logical {
            self.data.push(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_read() {
        let mut fs = FileSystem::new();
        let id = fs.create_file("/etc/config", 0, PERM_RW).unwrap();
        fs.write(id, b"key=value").unwrap();
        assert_eq!(fs.read(id).unwrap(), b"key=value");
    }

    #[test]
    fn test_ls_prefix_scan() {
        let mut fs = FileSystem::new();
        fs.create_dir("/home", 0, PERM_RWX).unwrap();
        fs.create_file("/home/readme.md", 0, PERM_RW).unwrap();
        fs.create_file("/home/notes.txt", 0, PERM_RW).unwrap();
        fs.create_dir("/etc", 0, PERM_RWX).unwrap();
        fs.create_file("/etc/config", 0, PERM_RW).unwrap();

        let home_files = fs.ls("/home/");
        assert_eq!(home_files.len(), 2);

        let root_entries = fs.ls("/");
        assert_eq!(root_entries.len(), 2); // /home and /etc
    }

    #[test]
    fn test_ls_no_deep_children() {
        let mut fs = FileSystem::new();
        fs.create_dir("/a", 0, PERM_RWX).unwrap();
        fs.create_dir("/a/b", 0, PERM_RWX).unwrap();
        fs.create_file("/a/b/file.txt", 0, PERM_RW).unwrap();
        fs.create_file("/a/top.txt", 0, PERM_RW).unwrap();

        let a_contents = fs.ls("/a/");
        let names: Vec<&str> = a_contents.iter().map(|(_, n)| *n).collect();
        assert!(names.contains(&"/a/b"));
        assert!(names.contains(&"/a/top.txt"));
        assert!(!names.contains(&"/a/b/file.txt"));
    }

    #[test]
    fn test_find_is_grep() {
        let mut fs = FileSystem::new();
        fs.create_file("/home/readme.md", 0, PERM_RW).unwrap();
        fs.create_file("/docs/readme.md", 0, PERM_RW).unwrap();
        fs.create_file("/etc/config", 0, PERM_RW).unwrap();

        let results = fs.find("readme");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_delete_tombstones() {
        let mut fs = FileSystem::new();
        let id = fs.create_file("/tmp/scratch", 0, PERM_RW).unwrap();
        fs.write(id, b"temp data").unwrap();

        fs.delete(id).unwrap();
        assert!(fs.read(id).is_err());
        assert!(fs.find_path("/tmp/scratch").is_none());
    }

    #[test]
    fn test_rename_is_string_edit() {
        let mut fs = FileSystem::new();
        let id = fs.create_file("/old/path.txt", 0, PERM_RW).unwrap();
        fs.write(id, b"data").unwrap();

        fs.rename(id, "/new/path.txt").unwrap();
        assert_eq!(fs.get_name(id), Some("/new/path.txt"));
        assert_eq!(fs.read(id).unwrap(), b"data");
        assert!(fs.find_path("/old/path.txt").is_none());
    }

    #[test]
    fn test_duplicate_path_rejected() {
        let mut fs = FileSystem::new();
        fs.create_file("/a.txt", 0, PERM_RW).unwrap();
        assert!(fs.create_file("/a.txt", 0, PERM_RW).is_err());
    }

    #[test]
    fn test_compact_preserves_files() {
        let mut fs = FileSystem::new();
        let id0 = fs.create_file("/a", 0, PERM_RW).unwrap();
        let id1 = fs.create_file("/b", 0, PERM_RW).unwrap();
        let id2 = fs.create_file("/c", 0, PERM_RW).unwrap();
        fs.write(id0, b"aaa").unwrap();
        fs.write(id2, b"ccc").unwrap();

        fs.delete(id1).unwrap();
        fs.compact();

        assert_eq!(fs.file_count(), 2);
        assert_eq!(fs.read(id0).unwrap(), b"aaa");
        assert_eq!(fs.read(id2).unwrap(), b"ccc");
        assert_eq!(fs.get_name(id0), Some("/a"));
        assert_eq!(fs.get_name(id2), Some("/c"));
    }

    #[test]
    fn test_identity_never_changes() {
        let mut fs = FileSystem::new();
        let id = fs.create_file("/stable.txt", 0, PERM_RW).unwrap();
        fs.write(id, b"original").unwrap();

        // Create and delete many files
        for i in 0..10 {
            let tmp = fs.create_file(&alloc::format!("/tmp/{}", i), 0, PERM_RW).unwrap();
            fs.delete(tmp).unwrap();
        }
        fs.compact();

        // Original ID still works
        assert_eq!(fs.get_name(id), Some("/stable.txt"));
        assert_eq!(fs.read(id).unwrap(), b"original");
    }

    #[test]
    fn test_type_tracking() {
        let mut fs = FileSystem::new();
        let dir = fs.create_dir("/bin", 0, PERM_RWX).unwrap();
        let file = fs.create_file("/bin/sh", 0, PERM_RWX).unwrap();
        assert_eq!(fs.get_type(dir), Some(TYPE_DIR));
        assert_eq!(fs.get_type(file), Some(TYPE_FILE));
    }
}
