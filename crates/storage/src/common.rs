pub const PAGE_SIZE: usize = 8192;
pub const HEADER_SIZE: usize = 10;
pub const SLOT_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u32);
