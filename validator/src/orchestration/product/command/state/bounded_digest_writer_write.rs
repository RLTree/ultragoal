impl Write for BoundedDigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.written) {
            self.exceeded = true;
            return Err(io::Error::other("bounded state commitment exceeded"));
        }
        self.hasher.update(bytes);
        self.written += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn action_priority(operation: RootOperation) -> u8 {
    match operation {
        RootOperation::Reconcile => 0,
        RootOperation::Recover => 1,
        RootOperation::Resume => 2,
    }
}
