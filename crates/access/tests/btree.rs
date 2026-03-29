use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("btree");
    assert!(dir.path().exists());
}
