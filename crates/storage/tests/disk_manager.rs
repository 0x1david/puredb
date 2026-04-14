use storage::common::PAGE_SIZE;
use storage::common::PageId;
use storage::disk_manager::{DiskManager, FileDiskManager};

struct CleanupFile(std::path::PathBuf);

impl Drop for CleanupFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

fn temp_path(name: &str) -> CleanupFile {
    let path = std::env::temp_dir().join(format!("puredb_{}_{}", name, std::process::id()));
    CleanupFile(path)
}

#[test]
fn test_disk_manager() {
    let guard = temp_path("dm");
    let path = &guard.0;

    {
        let mut fdm = FileDiskManager::new(path).unwrap();
        for i in 0u8..7 {
            let id = fdm.allocate_page();
            fdm.write_page(id, &[i; PAGE_SIZE]).unwrap();
        }
    }

    let mut fdm = FileDiskManager::open(path).unwrap();
    for i in 7u8..10 {
        let id = fdm.allocate_page();
        fdm.write_page(id, &[i; PAGE_SIZE]).unwrap();
    }

    for i in 0u8..10 {
        assert_eq!(fdm.read_page(PageId(i as u32)).unwrap(), [i; PAGE_SIZE]);
    }
}

#[test]
fn test_read_unallocated_page() {
    let guard = temp_path("unalloc");
    let fdm = FileDiskManager::new(&guard.0).unwrap();
    assert!(fdm.read_page(PageId(0)).is_err());
}
