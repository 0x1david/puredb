use storage::{DiskManager, FileDiskManager, PAGE_SIZE};

#[test]
fn test_disk_manager() {
    let p1;
    let p2;
    let p3;
    let d1: [u8; PAGE_SIZE];
    let d2: [u8; PAGE_SIZE];
    let d3: [u8; PAGE_SIZE];
    let path = std::env::temp_dir().join(format!("puredb_test_{}", std::process::id()));

    {
        let mut fdm = FileDiskManager::new(&path).unwrap();
        assert!(path.exists());

        d1 = [0xAA; PAGE_SIZE];
        d2 = [0xBB; PAGE_SIZE];
        d3 = [0xCC; PAGE_SIZE];

        p1 = fdm.allocate_page();
        fdm.write_page(p1, &d1).unwrap();

        p2 = fdm.allocate_page();
        fdm.write_page(p2, &d2).unwrap();

        p3 = fdm.allocate_page();
        fdm.write_page(p3, &d3).unwrap();
    }

    let mut fdm = FileDiskManager::open(&path).unwrap();

    let d4 = [0xDD; PAGE_SIZE];
    let p4 = fdm.allocate_page();

    fdm.write_page(p4, &d4).unwrap();

    assert_eq!(fdm.read_page(p1).unwrap(), d1);
    assert_eq!(fdm.read_page(p2).unwrap(), d2);
    assert_eq!(fdm.read_page(p3).unwrap(), d3);
    assert_eq!(fdm.read_page(p4).unwrap(), d4);

    std::fs::remove_file(&path).ok();
}
