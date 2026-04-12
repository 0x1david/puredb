//### 1.2 — Page Layout
//Define the binary format of a page. A slotted-page design:
//
//- Page header: page ID, number of tuples, free space pointers
//- Slot array growing forward, tuple data growing backward
//- Helpers to insert/read/delete a variable-length byte slice within a page
//
//**Test:** Insert entries into a page until full, read them back by slot index, delete one, compact.
//
use crate::common::{PageId, PAGE_SIZE};
use std::mem::size_of;

const HEADER_ID: usize = 0;
const HEADER_NUM_SLOTS: usize = HEADER_ID + size_of::<u32>();
const HEADER_FREE_START: usize = HEADER_NUM_SLOTS + size_of::<u16>();
const HEADER_FREE_END: usize = HEADER_FREE_START + size_of::<u16>();
const HEADER_SIZE: usize = HEADER_FREE_END + size_of::<u16>();
const SLOT_SIZE: usize = 4;
const SLOT_DATA_SIZE: usize = 0;
const SLOT_DATA_OFFSET: usize = 2;

fn read_u16(b: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([b[offset], b[offset + 1]])
}

fn read_u32(b: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([b[offset], b[offset + 1], b[offset + 2], b[offset + 3]])
}
fn write_u16(b: &mut [u8], offset: usize, v: u16) {
    b[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
}

fn write_u32(b: &mut [u8], offset: usize, v: u32) {
    b[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

struct Header<B: AsRef<[u8]>>(B);

impl<B: AsRef<[u8]>> Header<B> {
    fn id(&self) -> PageId {
        PageId(read_u32(self.0.as_ref(), HEADER_ID))
    }
    fn num_slots(&self) -> u16 {
        read_u16(self.0.as_ref(), HEADER_NUM_SLOTS)
    }
    fn free_start(&self) -> u16 {
        read_u16(self.0.as_ref(), HEADER_FREE_START)
    }
    fn free_end(&self) -> u16 {
        read_u16(self.0.as_ref(), HEADER_FREE_END)
    }
    // Log Sequence Number
    // Checksum
    // Flags
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Header<B> {
    fn set_num_slots(&mut self, v: u16) {
        write_u16(self.0.as_mut(), HEADER_NUM_SLOTS, v);
    }
    fn set_free_start(&mut self, v: u16) {
        write_u16(self.0.as_mut(), HEADER_FREE_START, v);
    }
    fn set_free_end(&mut self, v: u16) {
        write_u16(self.0.as_mut(), HEADER_FREE_END, v);
    }
}

struct Slot<B: AsRef<[u8]>>(B);

impl<B: AsRef<[u8]>> Slot<B> {
    fn size(&self) -> u16 {
        read_u16(self.0.as_ref(), SLOT_DATA_SIZE)
    }
    fn offset(&self) -> u16 {
        read_u16(self.0.as_ref(), SLOT_DATA_OFFSET)
    }
    fn is_dead(&self) -> bool {
        self.offset() == 0
    }
    fn is_alive(&self) -> bool {
        self.offset() != 0
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Slot<B> {
    fn set_size(&mut self, v: u16) {
        write_u16(self.0.as_mut(), SLOT_DATA_SIZE, v);
    }
    fn set_offset(&mut self, v: u16) {
        write_u16(self.0.as_mut(), SLOT_DATA_OFFSET, v);
    }
    fn mark_dead(&mut self) {
        self.set_offset(0);
    }
}

pub type SlotIndex = u16;
type RecordID = (PageId, SlotIndex);

pub struct Page<'a>(&'a mut [u8; PAGE_SIZE]);

impl<'a> Page<'a> {
    pub fn new(buf: &'a mut [u8; PAGE_SIZE]) -> Self {
        let mut p = Page(buf);
        let mut h = p.header_mut();
        h.set_free_start(HEADER_SIZE as u16);
        h.set_free_end(PAGE_SIZE as u16);
        p
    }
    fn header(&self) -> Header<&[u8]> {
        Header(&self.0[..HEADER_SIZE])
    }
    fn header_mut(&mut self) -> Header<&mut [u8]> {
        Header(&mut self.0[..HEADER_SIZE])
    }
    fn slot(&self, idx: SlotIndex) -> Slot<&[u8]> {
        let s = HEADER_SIZE + (idx as usize * SLOT_SIZE);
        Slot(&self.0[s..s + SLOT_SIZE])
    }
    fn slot_mut(&mut self, idx: SlotIndex) -> Slot<&mut [u8]> {
        let s = HEADER_SIZE + (idx as usize * SLOT_SIZE);
        Slot(&mut self.0[s..s + SLOT_SIZE])
    }
    fn tuple(&self, s: Slot<&[u8]>) -> &[u8] {
        let start = s.offset() as usize;
        let end = start + s.size() as usize;
        &self.0[start..end]
    }
    fn tuple_mut(&mut self, s: Slot<&mut [u8]>) -> &mut [u8] {
        let start = s.offset() as usize;
        let end = start + s.size() as usize;
        &mut self.0[start..end]
    }

    fn has_enough_space(&self, size: usize) -> bool {
        let h = self.header();
        let free_size = (h.free_end() - h.free_start()) as usize;
        free_size >= (size + SLOT_SIZE)
    }

    /// Returns new free space end
    fn write_to_free_space(&mut self, data: &[u8]) -> u16 {
        let end = self.header().free_end() as usize;

        let start = end - data.len();
        self.0[start..end].copy_from_slice(data);
        start as u16
    }

    fn add_slot(&mut self, offset: u16, size: u16) -> u16 {
        let mut header = self.header_mut();
        let num_slots = header.num_slots();
        header.set_num_slots(num_slots + 1);
        let start = HEADER_SIZE + (num_slots as usize * SLOT_SIZE);
        let new_start = start + SLOT_SIZE;

        let mut slot = Slot(&mut self.0[start..new_start]);
        slot.set_size(size);
        slot.set_offset(offset);

        new_start as u16
    }

    pub fn insert(&mut self, data: &[u8]) -> Option<SlotIndex> {
        let size = data.len();

        if !self.has_enough_space(size) {
            self.compact();

            if !self.has_enough_space(size) {
                return None;
            }
        }

        let new_end = self.write_to_free_space(data);
        let new_start = self.add_slot(new_end, size as u16);

        let mut h = self.header_mut();
        h.set_free_start(new_start);
        h.set_free_end(new_end);
        Some(h.num_slots() - 1)
    }

    pub fn read(&self, idx: SlotIndex) -> Option<&[u8]> {
        assert!(idx < self.header().num_slots(), "slot index {idx} out of bounds (num_slots: {})", self.header().num_slots());
        let s = self.slot(idx);
        s.is_alive().then(|| self.tuple(s))
    }

    pub fn delete(&mut self, idx: SlotIndex) {
        assert!(idx < self.header().num_slots(), "slot index {idx} out of bounds (num_slots: {})", self.header().num_slots());
        self.slot_mut(idx).mark_dead();
    }

    pub fn compact(&mut self) {
        let mut back_ptr = PAGE_SIZE;

        for i in 0..self.header().num_slots() {
            let mut slot = self.slot_mut(i);
            if slot.is_dead() {
                continue;
            }

            let offset = slot.offset() as usize;
            let size = slot.size() as usize;

            back_ptr -= size;
            slot.set_offset(back_ptr as u16);
            self.0.copy_within(offset..offset + size, back_ptr);
        }
        self.header_mut().set_free_end(back_ptr as u16);
    }
}
