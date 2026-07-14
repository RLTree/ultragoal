use super::*;

pub(crate) const ORIGINAL: &[u8] = b"original";
pub(crate) const FOREIGN: &[u8] = b"foreign!";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReadEvent {
    BeforeAncestorOpen,
    AfterAncestorOpen,
    AfterFirstLeafRead,
    AfterAttachmentRevalidated,
    BeforeFinalRootCheck,
    BeforeFinalPathBinding,
    AfterFinalPathBinding,
}

pub(crate) type Observer = Box<dyn FnMut(ReadEvent)>;

thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = RefCell::new(None);
}

pub(crate) fn observe(event: ReadEvent) {
    OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow_mut().as_mut() {
            observer(event);
        }
    });
}

pub(crate) fn with_observer<T>(
    observer: impl FnMut(ReadEvent) + 'static,
    action: impl FnOnce() -> T,
) -> T {
    OBSERVER.with(|slot| {
        assert!(slot.replace(Some(Box::new(observer))).is_none());
    });
    let result = action();
    OBSERVER.with(|slot| slot.borrow_mut().take());
    result
}

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) alternate: PathBuf,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container = PathBuf::from(format!(
            "/tmp/hul-fit-ancestor-{label}-{}-{nonce}",
            std::process::id()
        ));
        let root = container.join("repo");
        let alternate = container.join("alternate");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&alternate).unwrap();
        Self {
            container,
            root,
            alternate,
        }
    }

    pub(crate) fn write_root(&self, relative: &str, bytes: &[u8]) {
        write(&self.root.join(relative), bytes);
    }

    pub(crate) fn write_alternate(&self, relative: &str, bytes: &[u8]) {
        write(&self.alternate.join(relative), bytes);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

pub(crate) fn assert_fail_closed(result: Result<Option<Vec<u8>>, FitError>) {
    let id = result.expect_err("detached bytes returned").id();
    assert!(
        matches!(id, FitErrorId::StaleBinding | FitErrorId::UnsafeObject),
        "{id:?}"
    );
}

pub(crate) fn observed_substitution(
    fixture: &Fixture,
    relative: &str,
    event: ReadEvent,
    occurrence: usize,
    active_relative: &str,
    alternate_relative: &str,
) {
    let workspace = Workspace::open(&fixture.root).unwrap();
    let active = fixture.root.join(active_relative);
    let alternate = fixture.alternate.join(alternate_relative);
    let detached = fixture.container.join("detached");
    let mut observed_occurrences = 0;
    let result = with_observer(
        move |observed_event| {
            if observed_event == event {
                if observed_occurrences == occurrence {
                    fs::rename(&active, &detached).unwrap();
                    fs::rename(&alternate, &active).unwrap();
                }
                observed_occurrences += 1;
            }
        },
        || workspace.read_file(&CanonicalPath::parse(relative).unwrap(), 1024),
    );
    assert_fail_closed(result);
    assert_eq!(fs::read(fixture.root.join(relative)).unwrap(), FOREIGN);
}

pub(crate) fn case(
    label: &str,
    relative: &str,
    event: ReadEvent,
    occurrence: usize,
    active: &str,
    alternate_file: &str,
    alternate: &str,
) {
    let fixture = Fixture::new(label);
    fixture.write_root(relative, ORIGINAL);
    fixture.write_alternate(alternate_file, FOREIGN);
    observed_substitution(&fixture, relative, event, occurrence, active, alternate);
}

#[test]
pub(crate) fn every_ordered_pre_binding_substitution_fails_closed() {
    use ReadEvent::*;
    let (bo, ao, lr) = (BeforeAncestorOpen, AfterAncestorOpen, AfterFirstLeafRead);
    let (ar, rr, fb) = (
        AfterAttachmentRevalidated,
        BeforeFinalRootCheck,
        BeforeFinalPathBinding,
    );
    case("before-open", "a/b/FILE", bo, 0, "a", "b/FILE", "");
    case("traversal", "a/b/FILE", ao, 0, "a", "b/FILE", "");
    case("leaf", "a/b/FILE", lr, 0, "a/b/FILE", "FILE", "FILE");
    case("deepest", "a/b/c/FILE", ar, 0, "a/b/c", "FILE", "");
    case("intermediate", "a/b/c/FILE", ar, 1, "a/b", "c/FILE", "");
    case("root-most", "a/b/c/FILE", ar, 2, "a", "b/c/FILE", "");
    case("root", "a/b/FILE", rr, 0, "", "a/b/FILE", "");
    case("depth-one-bind", "a/FILE", fb, 0, "a", "FILE", "");
    case("deep-bind", "a/b/c/FILE", fb, 0, "a", "b/c/FILE", "");
}
