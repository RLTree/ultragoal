#[cfg(test)]
fn run_fault(point: FaultPoint, path: &Path) -> Result<(), ()> {
    let armed = FAULT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|(expected, _, _)| *expected == point)
        {
            slot.take()
        } else {
            None
        }
    });
    if let Some((_, fail, hook)) = armed {
        hook(path);
        if fail {
            return Err(());
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn run_fault(_point: FaultPoint, _path: &Path) -> Result<(), ()> {
    Ok(())
}
