use super::{CommandDescriptor, OptionName, ValueKind};

pub(super) fn required_args(
    descriptor: &CommandDescriptor,
    omit: Option<OptionName>,
) -> Vec<String> {
    let mut args = vec![descriptor.command.group().as_str().to_owned()];
    if let Some(subcommand) = descriptor.subcommand {
        args.push(subcommand.to_owned());
    }
    for option in descriptor
        .options
        .iter()
        .filter(|option| option.required && Some(option.name) != omit)
    {
        push_option(&mut args, option.name, option.kind);
    }
    args
}

pub(super) fn push_option(args: &mut Vec<String>, name: OptionName, kind: ValueKind) {
    args.push(name.as_str().to_owned());
    match kind {
        ValueKind::Flag => {}
        ValueKind::Identifier => args.push("candidate-1".to_owned()),
        ValueKind::RepositoryTarget => args.push(".".to_owned()),
        ValueKind::RelativePath => args.push("artifacts/result.json".to_owned()),
        ValueKind::HostPath => args.push("/package".to_owned()),
    }
}
