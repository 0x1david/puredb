use storage::common::PAGE_SIZE;
use storage::common::PageId;
use storage::disk_manager::{DiskManager, FileDiskManager};
use storage::page::{Page, SlotIndex};

#[test]
fn test_disk_manager() {
    let path = std::env::temp_dir().join(format!("puredb_test_{}", std::process::id()));

    {
        let mut fdm = FileDiskManager::new(&path).unwrap();
        for i in 0u8..7 {
            let id = fdm.allocate_page();
            fdm.write_page(id, &[i; PAGE_SIZE]).unwrap();
        }
    }

    let mut fdm = FileDiskManager::open(&path).unwrap();
    for i in 7u8..10 {
        let id = fdm.allocate_page();
        fdm.write_page(id, &[i; PAGE_SIZE]).unwrap();
    }

    for i in 0u8..10 {
        assert_eq!(fdm.read_page(PageId(i as u32)).unwrap(), [i; PAGE_SIZE]);
    }

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_page() {
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
                assert_eq!(page_data, None)
            } else {
                assert_eq!(orig_data.to_le_bytes(), page_data.unwrap())
            }
        });

    page.compact();

    for i in 0u8..10 {
        let data = page.read(i as SlotIndex);
        if i == 5 {
            assert_eq!(data, None);
        } else {
            assert_eq!(data.unwrap(), i.to_le_bytes());
        }
    }
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

    assert_eq!(&buf[PAGE_SIZE - 3..PAGE_SIZE], b"aaa");
    assert_eq!(&buf[PAGE_SIZE - 6..PAGE_SIZE - 3], b"ccc");
}
