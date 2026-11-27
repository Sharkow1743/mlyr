#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SyncKind {
    #[default]
    None,
    Line,
    Word,
    Syllable,
    Character,
}
