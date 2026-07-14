use super::*;
use crate::routine_work::{
    CHILD_CAPABILITY_ENV, CHILD_CAPABILITY_FD, MAX_CHILD_CAPABILITY_BYTES, RoutineChildCapability,
};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

pub(crate) struct ChildAuthorityChannel {
    parent: UnixStream,
    child: UnixStream,
}

impl ChildAuthorityChannel {
    pub(crate) fn new() -> Result<Self, RoutineError> {
        let (parent, child) = UnixStream::pair()
            .map_err(|_| mediator_error("mediator-child-capability-channel-failed"))?;
        let timeout = Some(Duration::from_secs(2));
        parent
            .set_read_timeout(timeout)
            .and_then(|()| parent.set_write_timeout(timeout))
            .map_err(|_| mediator_error("mediator-child-capability-channel-failed"))?;
        Ok(Self { parent, child })
    }

    pub(crate) fn install(&self, command: &mut Command) {
        command.env(CHILD_CAPABILITY_ENV, CHILD_CAPABILITY_FD.to_string());
        let source_fd = self.child.as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                if source_fd != CHILD_CAPABILITY_FD
                    && libc::dup2(source_fd, CHILD_CAPABILITY_FD) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                let flags = libc::fcntl(CHILD_CAPABILITY_FD, libc::F_GETFD);
                if flags < 0
                    || libc::fcntl(
                        CHILD_CAPABILITY_FD,
                        libc::F_SETFD,
                        flags & !libc::FD_CLOEXEC,
                    ) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    pub(crate) fn authorize(
        mut self,
        child_pid: u32,
        binding: ChildCapabilityBinding<'_>,
        program: &PinnedExecutable,
    ) -> Result<(), RoutineError> {
        drop(self.child);
        let mut nonce = [0_u8; 32];
        getrandom::fill(&mut nonce)
            .map_err(|_| mediator_error("mediator-child-capability-random-failed"))?;
        let process_session_id = unsafe { libc::getsid(0) };
        let (changed_seconds, changed_nanos) = program.identity_changed();
        let mut capability = RoutineChildCapability {
            schema_version: "RoutineChildCapability-v1".to_owned(),
            behavior_id: binding.behavior_id.to_owned(),
            request_id: binding.request_id.to_owned(),
            protocol_id: binding.protocol_id.to_owned(),
            intent_id: binding.intent_id.to_owned(),
            node_id: binding.node_id.to_owned(),
            grant_session_id: binding.grant_session_id.to_owned(),
            grant_id: binding.grant_id.to_owned(),
            reservation_marker: binding.reservation_marker.to_owned(),
            parent_pid: std::process::id() as i32,
            child_pid: child_pid as i32,
            process_session_id,
            program_path_hex: binding.program_path_hex.to_owned(),
            program_sha256: program.sha256().to_owned(),
            program_byte_length: program.identity_length(),
            program_unix_mode: program.identity_mode(),
            program_device: program.identity_device(),
            program_inode: program.identity_inode(),
            program_changed_seconds: changed_seconds,
            program_changed_nanos: changed_nanos,
            framed_input_sha256: binding.framed_input_sha256.to_owned(),
            nonce_hex: nonce.iter().map(|byte| format!("{byte:02x}")).collect(),
            capability_seal: String::new(),
        };
        capability.seal(binding.secret)?;
        let bytes = capability.encode()?;
        if bytes.len() > MAX_CHILD_CAPABILITY_BYTES {
            return Err(mediator_error("mediator-child-capability-oversize"));
        }
        self.parent
            .write_all(binding.secret)
            .and_then(|()| self.parent.write_all(&(bytes.len() as u32).to_be_bytes()))
            .and_then(|()| self.parent.write_all(&bytes))
            .map_err(|_| mediator_error("mediator-child-capability-send-failed"))?;
        let mut acknowledgement = [0_u8; 71];
        self.parent
            .read_exact(&mut acknowledgement)
            .map_err(|_| mediator_error("mediator-child-capability-ack-failed"))?;
        if acknowledgement != capability.acknowledgement().as_bytes() {
            return Err(mediator_error("mediator-child-capability-ack-invalid"));
        }
        Ok(())
    }
}
