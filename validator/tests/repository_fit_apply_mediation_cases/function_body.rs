pub(crate) fn function_body<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing function {marker}"));
    let body = start
        + source[start..]
            .find('{')
            .unwrap_or_else(|| panic!("missing function body {marker}"));
    let mut depth = 0_usize;
    for (offset, byte) in source[body..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=body + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated function {marker}");
}

pub(crate) fn declaration<'a>(source: &'a str, marker: &str) -> (&'a str, &'a str) {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing {marker}"));
    let end = start + source[start..].find("\n}").unwrap() + 2;
    let prefix_start = source[..start]
        .rfind("\n\n")
        .map_or(0, |boundary| boundary + 2);
    (&source[prefix_start..start], &source[start..end])
}
