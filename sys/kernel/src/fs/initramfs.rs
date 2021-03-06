// Minimal Initial Ram Filesystem

use super::*;
use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};
use byteorder::*;
use core::{intrinsics::copy_nonoverlapping, ptr::slice_from_raw_parts_mut};
use megstd::io;

pub(super) struct InitRamfs {
    data: Box<[u8]>,
    dir: Box<[MyFsDirEntry]>,
}

impl InitRamfs {
    const MAGIC_CURRENT: u32 = 0x0001beef;
    const SIZE_OF_RAW_DIR: usize = 32;
    const OFFSET_DATA: usize = 16;

    /// SAFETY: Must guarantee the existence of the data.
    pub(super) unsafe fn from_static(base: usize, len: usize) -> Option<Self> {
        let boxed = Box::from_raw(slice_from_raw_parts_mut(base as *mut u8, len));
        let mut dir = Vec::new();
        Self::parse_header(&boxed, &mut dir).then(|| Self {
            data: boxed,
            dir: dir.into_boxed_slice(),
        })
    }

    #[inline]
    fn parse_header(data: &Box<[u8]>, dir: &mut Vec<MyFsDirEntry>) -> bool {
        if LE::read_u32(&data[0..4]) != Self::MAGIC_CURRENT {
            return false;
        }

        let dir_base = LE::read_u32(&data[4..8]) as usize;
        let n_dirent = LE::read_u32(&data[8..12]) as usize;

        for index in 0..n_dirent {
            let dir_offset = dir_base + index * Self::SIZE_OF_RAW_DIR;
            let name_len = data[dir_offset] as usize;
            let name =
                String::from_utf8(data[dir_offset + 1..dir_offset + name_len + 1].to_owned())
                    .unwrap_or("#NAME?".to_owned());
            dir.push(MyFsDirEntry {
                inode: unsafe { NonZeroINodeType::new_unchecked(index as INodeType + 1) },
                name,
                offset: LE::read_u32(&data[dir_offset + 0x18..dir_offset + 0x1C]) as usize,
                size: LE::read_u32(&data[dir_offset + 0x1C..dir_offset + 0x20]) as usize,
            });
        }

        true
    }

    #[inline]
    pub fn read_dir(&self, index: usize) -> Option<FsRawDirEntry> {
        self.dir.get(index).map(|v| v.into())
    }

    #[inline]
    pub fn find_file(&self, lpc: &str) -> Option<NonZeroINodeType> {
        self.dir.iter().find(|v| lpc == v.name).map(|v| v.inode)
    }

    #[inline]
    pub fn stat(&self, inode: NonZeroINodeType) -> Option<FsRawMetaData> {
        self.get_file(inode).map(|v| v.into())
    }

    #[inline]
    fn get_file(&self, inode: NonZeroINodeType) -> Option<&MyFsDirEntry> {
        self.dir.get(inode.get() as usize - 1)
    }

    pub fn read_data(
        &self,
        inode: Option<NonZeroINodeType>,
        offset: OffsetType,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        let dir_ent = match inode.and_then(|v| self.get_file(v)) {
            Some(v) => v,
            None => return Err(io::ErrorKind::NotFound.into()),
        };
        if offset > dir_ent.size as OffsetType {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let size_left = dir_ent.size as OffsetType - offset;
        let count = usize::min(size_left as usize, buf.len());
        if count > 0 {
            unsafe {
                let src = (&self.data[0] as *const _ as usize
                    + Self::OFFSET_DATA
                    + dir_ent.offset
                    + offset as usize) as *const u8;
                let dst = &mut buf[0] as *mut _;
                copy_nonoverlapping(src, dst, count);
            }
        }
        Ok(count)
    }
}

struct MyFsDirEntry {
    inode: NonZeroINodeType,
    name: String,
    offset: usize,
    size: usize,
}

impl From<&MyFsDirEntry> for FsRawDirEntry {
    fn from(src: &MyFsDirEntry) -> Self {
        FsRawDirEntry::new(src.inode, src.name.clone(), Some(src.into()))
    }
}

impl From<&MyFsDirEntry> for FsRawMetaData {
    fn from(src: &MyFsDirEntry) -> Self {
        FsRawMetaData::new(src.size as OffsetType)
    }
}
