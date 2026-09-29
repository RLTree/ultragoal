pub(crate) fn active_command_groups() -> BTreeSet<&'static str> {
    let represented = BINDINGS
        .iter()
        .map(|binding| binding.command.group())
        .collect::<BTreeSet<Group>>();
    represented
        .into_iter()
        .filter(|group| {
            catalog()
                .iter()
                .filter(|descriptor| descriptor.command.group() == *group)
                .all(|descriptor| bind_command(descriptor.command, descriptor.effect).is_some())
        })
        .map(Group::as_str)
        .collect()
}
