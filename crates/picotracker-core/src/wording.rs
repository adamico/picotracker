//! Wording shared by the messages and warnings of every module.

/// A number of things with their noun, which takes an S unless there is exactly one.
pub(crate) fn counted(count: usize, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "S" };
    format!("{count} {noun}{plural}")
}
