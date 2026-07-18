pub(crate) fn declaration<'a>(source: &'a str, marker: &str) -> (&'a str, String) {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing {marker}"));
    let prefix_start = source[..start]
        .rfind("\n\n")
        .map_or(0, |boundary| boundary + 2);
    let declaration = source[start..]
        .lines()
        .take_while(|line| line.trim() != "}")
        .chain(std::iter::once("}"))
        .collect::<Vec<_>>()
        .join("\n");
    (&source[prefix_start..start], declaration)
}
