use storage::common::{HEADER_SIZE, PAGE_SIZE, SLOT_SIZE};
use storage::page::{Page, SlotIndex};

fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn page_num_slots(buf: &[u8]) -> u16 {
    read_u16(buf, 4)
}

fn page_free_start(buf: &[u8]) -> u16 {
    read_u16(buf, 6)
}

fn page_free_end(buf: &[u8]) -> u16 {
    read_u16(buf, 8)
}

#[test]
fn test_insert_read_delete() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    let indexes: Vec<SlotIndex> = (0u8..10)
        .map(|data| page.insert(&data.to_le_bytes()).unwrap())
        .collect();

    let del = indexes[5];
    page.delete(del);

    indexes
        .into_iter()
        .map(|it| page.read(it))
        .zip(0u8..10)
        .for_each(|(page_data, orig_data)| {
            if orig_data == 5u8 {
                assert_eq!(page_data, None);
                return;
            }
            assert_eq!(orig_data.to_le_bytes(), page_data.unwrap())
        });

    page.compact();

    // Slot indices must remain stable across compaction — the slot array
    // doesn't move, only the underlying tuple data gets defragmented.
    for i in 0u8..10 {
        let data = page.read(i as SlotIndex);
        if i == 5 {
            assert_eq!(data, None);
            continue;
        }
        assert_eq!(data.unwrap(), i.to_le_bytes());
    }
}

#[test]
fn test_zero_length_insert() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    let idx = page.insert(&[]).unwrap();
    assert_eq!(page.read(idx).unwrap(), &[] as &[u8]);
}

#[test]
fn test_page_full() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    const DATA_SIZE: usize = 8;
    let data = [0xAB; DATA_SIZE];
    let mut count = 0u16;
    while page.insert(&data).is_some() {
        count += 1;
    }
    assert_eq!(
        count,
        ((PAGE_SIZE - HEADER_SIZE) / (DATA_SIZE + SLOT_SIZE)) as u16
    );

    for i in 0..count {
        assert_eq!(page.read(i).unwrap(), data);
    }
}

#[test]
fn test_large_tuple() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    // Largest possible single tuple: free space minus one slot
    let max_data = vec![0xCD; PAGE_SIZE - HEADER_SIZE - SLOT_SIZE];
    let idx = page.insert(&max_data).unwrap();
    assert_eq!(page.read(idx).unwrap(), &max_data[..]);

    assert_eq!(page.insert(&[0]), None);
}

#[test]
#[should_panic]
fn test_insert_oversized_panics() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);
    // One byte over the absolute maximum — can never fit regardless of free space
    let oversized = vec![0u8; PAGE_SIZE - HEADER_SIZE - SLOT_SIZE + 1];
    page.insert(&oversized);
}

#[test]
fn test_delete_first_and_last() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    let slots: Vec<SlotIndex> = (0u8..5).map(|i| page.insert(&[i]).unwrap()).collect();

    page.delete(slots[0]);
    page.delete(slots[4]);

    assert_eq!(page.read(slots[0]), None);
    assert_eq!(page.read(slots[4]), None);
    for &idx in &slots[1..4] {
        assert!(page.read(idx).is_some());
    }

    page.compact();

    assert_eq!(page.read(slots[0]), None);
    assert_eq!(page.read(slots[4]), None);
    assert_eq!(page.read(slots[1]).unwrap(), &[1]);
    assert_eq!(page.read(slots[2]).unwrap(), &[2]);
    assert_eq!(page.read(slots[3]).unwrap(), &[3]);
}

#[test]
fn test_delete_all_slots() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    let slots: Vec<SlotIndex> = (0u8..5).map(|i| page.insert(&[i; 100]).unwrap()).collect();

    for &idx in &slots {
        page.delete(idx);
    }
    for &idx in &slots {
        assert_eq!(page.read(idx), None);
    }

    page.compact();

    // All tuple data reclaimed, but slot entries remain in the array
    assert_eq!(page_free_end(buf), PAGE_SIZE as u16);
    assert_eq!(page_num_slots(buf), 5);
}

#[test]
fn test_compact() {
    let buf = &mut [0u8; PAGE_SIZE];
    {
        let mut page = Page::new(buf);
        page.insert(b"aaa").unwrap();
        let b = page.insert(b"bbb").unwrap();
        page.insert(b"ccc").unwrap();
        page.delete(b);
        page.compact();
    }

    // Physical layout: live tuples packed at end
    let aaa_end = PAGE_SIZE;
    let aaa_start = aaa_end - b"aaa".len();
    let ccc_start = aaa_start - b"ccc".len();
    assert_eq!(&buf[aaa_start..aaa_end], b"aaa");
    assert_eq!(&buf[ccc_start..aaa_start], b"ccc");

    // Header metadata consistency after compaction
    assert_eq!(page_num_slots(buf), 3);
    assert_eq!(page_free_start(buf), (HEADER_SIZE + 3 * SLOT_SIZE) as u16);
    assert_eq!(
        page_free_end(buf),
        (PAGE_SIZE - b"aaa".len() - b"ccc".len()) as u16
    );
}

#[test]
fn test_double_compact_cycle() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    // Round 1: insert, delete middle, compact
    let a = page.insert(b"alpha").unwrap();
    let b = page.insert(b"beta").unwrap();
    let c = page.insert(b"gamma").unwrap();
    page.delete(b);
    page.compact();

    assert_eq!(page.read(a).unwrap(), b"alpha");
    assert_eq!(page.read(b), None);
    assert_eq!(page.read(c).unwrap(), b"gamma");

    // Round 2: insert more, delete first, compact again
    let d = page.insert(b"delta").unwrap();
    page.delete(a);
    page.compact();

    assert_eq!(page.read(a), None);
    assert_eq!(page.read(b), None);
    assert_eq!(page.read(c).unwrap(), b"gamma");
    assert_eq!(page.read(d).unwrap(), b"delta");

    // Two live tuples packed at end
    assert_eq!(
        page_free_end(buf),
        (PAGE_SIZE - b"gamma".len() - b"delta".len()) as u16
    );
}

#[test]
fn test_compact_reclaims_space_for_insert() {
    let buf = &mut [0u8; PAGE_SIZE];
    let mut page = Page::new(buf);

    // Pick a size so one insert fits but two don't:
    // 2 * (DATA_SIZE + SLOT_SIZE) > PAGE_SIZE - HEADER_SIZE
    const DATA_SIZE: usize = (PAGE_SIZE - HEADER_SIZE) / 2 - SLOT_SIZE + 1;
    let big = vec![0xAA; DATA_SIZE];

    let a = page.insert(&big).unwrap();
    assert!(page.insert(&big).is_none(), "second insert should not fit");

    page.delete(a);
    page.compact();

    // Reclaimed DATA_SIZE bytes; a new slot costs SLOT_SIZE, leaving DATA_SIZE - SLOT_SIZE
    // bytes for data — enough to re-insert the same payload.
    let b = page.insert(&big).unwrap();
    assert_eq!(page.read(b).unwrap(), &big[..]);
}
