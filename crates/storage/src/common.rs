pub const PAGE_SIZE: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u32);
