fn store_fault(error: HostError) -> StoreFault {
    StoreFault::new(error.code())
}
