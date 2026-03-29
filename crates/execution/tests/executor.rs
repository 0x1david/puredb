use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("executor");
    assert!(dir.path().exists());
}
