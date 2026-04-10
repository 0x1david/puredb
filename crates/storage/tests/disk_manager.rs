use storage::disk_manager::{DiskManager, FileDiskManager, PAGE_SIZE, PageId};

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
