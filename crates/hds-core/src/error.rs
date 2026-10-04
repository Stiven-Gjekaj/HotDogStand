/// An error that the person can correct. Each message says what to change.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("The title is empty. Write a title.")]
    EmptyTitle,
    #[error("The title has {len} characters. Write {max} characters or fewer.")]
    TitleTooLong { len: usize, max: usize },
    #[error("The name is empty. Write a name.")]
    EmptyName,
    #[error("The color \"{0}\" is not a hex color. Write a color such as #3a6ea5.")]
    BadColor(String),
    #[error("The comment is empty. Write a comment.")]
    EmptyComment,
    #[error("\"{0}\" is not a status. Use open, in_progress, or closed.")]
    UnknownStatus(String),
    #[error("\"{0}\" is not a priority. Use low, normal, high, or urgent.")]
    UnknownPriority(String),
}
