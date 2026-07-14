struct ConcurrentOutput {
    rows: Arc<Mutex<Option<Vec<TreeObject>>>>,
    first_read: Arc<Barrier>,
    reads: usize,
}

impl MaterializeEffects for ConcurrentOutput {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.reads += 1;
        let observed = self.rows.lock().map_err(|_| ())?.clone();
        if self.reads == 1 {
            self.first_read.wait();
        }
        Ok(observed)
    }

    fn compare_exchange_tree(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        let mut rows = self.rows.lock().map_err(|_| ())?;
        let current = rows
            .as_deref()
            .map(tree_sha256)
            .transpose()
            .map_err(|_| ())?;
        if current.as_deref() != expected_sha256 {
            return Ok(false);
        }
        *rows = replacement.map(<[TreeObject]>::to_vec);
        Ok(true)
    }
}
