use std::{
    fs::{File, OpenOptions},
    io::{self},
    os::unix::fs::FileExt,
    path::Path,
};

use crate::common::{PageId, PAGE_SIZE};

impl PageId {
    fn from_file_len(v: u64) -> Self {
        PageId((v / PAGE_SIZE as u64) as u32)
    }
}

/// Only safe against overflow on 64-bit
fn offset(p: PageId) -> usize {
    p.0 as usize * PAGE_SIZE
}

pub trait DiskManager {
    fn allocate_page(&mut self) -> PageId;
    fn read_page(&self, p: PageId) -> io::Result<[u8; PAGE_SIZE]>;
    fn write_page(&mut self, p: PageId, buf: &[u8; PAGE_SIZE]) -> io::Result<()>;
}

pub struct FileDiskManager {
    next_page: PageId,
    file: File,
}

impl FileDiskManager {
    pub fn new(p: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(true)
            .open(p)?;

        Ok(Self {
            next_page: PageId(0),
            file,
        })
    }

    pub fn open(p: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(p)?;
        let next_page = PageId::from_file_len(file.metadata()?.len());
        Ok(Self { file, next_page })
    }
}

impl DiskManager for FileDiskManager {
    fn allocate_page(&mut self) -> PageId {
        let p = PageId(self.next_page.0);
        self.next_page.0 += 1;
        p
    }

    fn read_page(&self, p: PageId) -> std::io::Result<[u8; PAGE_SIZE]> {
        let mut buf = [0u8; PAGE_SIZE];
        self.file.read_exact_at(&mut buf, offset(p) as u64)?;
        Ok(buf)
    }

    fn write_page(&mut self, p: PageId, buf: &[u8; PAGE_SIZE]) -> io::Result<()> {
        self.file.write_all_at(buf, offset(p) as u64)
    }
}
