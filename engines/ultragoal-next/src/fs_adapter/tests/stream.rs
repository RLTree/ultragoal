use super::*;
use sha2::{Digest, Sha256};

#[test]
fn source_past_former_file_bound_streams_utf8_and_tail() {
    let tree = Tree::new();
    let mut source = vec![b'x'; 17 * 1024 * 1024 - 1];
    source.extend_from_slice("π\r\ntail-needle\n".as_bytes());
    fs::write(tree.at("large.py"), &source).unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let mut started = None;
    let mut hash = Sha256::new();
    let (mut offset, mut pages, mut tail) = (0u64, 0u64, false);
    let result = root.stream_source(
        b"large.py",
        |metadata| { started = Some((metadata.bytes, metadata.digest.clone())); Ok(()) },
        |at, text| {
            assert_eq!(at, offset);
            offset += text.len() as u64;
            pages += 1;
            tail |= text.contains("tail-needle");
            hash.update(text.as_bytes());
            Ok(())
        },
    ).unwrap();
    let digest = format!("{:x}", hash.finalize());
    assert_eq!(started, Some((source.len() as u64, digest.clone())));
    assert_eq!((result.bytes, result.pages, result.digest), (source.len() as u64, pages, digest));
    assert!(pages > 1 && tail);
}

#[test]
fn edit_during_delivery_invalidates_whole_logical_file() {
    let tree = Tree::new();
    let source = vec![b'x'; 5 * 1024 * 1024];
    fs::write(tree.at("source.txt"), &source).unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let mut changed = false;
    let outcome = root.stream_source(b"source.txt", |_| Ok(()), |_, _| {
        if !changed {
            fs::write(tree.at("source.txt"), vec![b'y'; source.len()]).unwrap();
            changed = true;
        }
        Ok(())
    });
    assert!(matches!(outcome, Err(StreamError::Changed)));
}
