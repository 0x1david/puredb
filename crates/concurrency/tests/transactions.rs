use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("transactions");
    assert!(dir.path().exists());
}
