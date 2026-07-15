use std::fs::File;
use std::io::{ErrorKind, Read};
use std::os::fd::{AsRawFd, FromRawFd};

const MAX_BYTES: usize = 16 * 1024;

pub(super) fn new_reader_writer() -> (File, File) {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let reader = unsafe { File::from_raw_fd(fds[0]) };
    let writer = unsafe { File::from_raw_fd(fds[1]) };
    set_reader_inheritable(&reader);
    (reader, writer)
}

pub(super) fn read_once(fd: i32) -> Result<Vec<u8>, String> {
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe { libc::fstat(fd, &mut stat) } != 0 || stat.st_mode & libc::S_IFMT != libc::S_IFIFO {
        return Err("capability fd was not the expected pipe".to_owned());
    }
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0 {
        return Err("capability fd could not be made nonblocking".to_owned());
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0_u8; 1024];
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) if bytes.len() + count <= MAX_BYTES => {
                bytes.extend_from_slice(&chunk[..count])
            }
            Ok(_) => return Err("capability envelope exceeded its bound".to_owned()),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                return Err("capability writer was not closed".to_owned());
            }
            Err(_) => return Err("capability pipe could not be read".to_owned()),
        }
    }
    Ok(bytes)
}

fn set_reader_inheritable(reader: &File) {
    let fd = reader.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) },
        0
    );
}
