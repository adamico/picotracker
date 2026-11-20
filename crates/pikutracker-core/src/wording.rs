pub(crate) fn counted(count: usize, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "S" };
    format!("{count} {noun}{plural}")
}
