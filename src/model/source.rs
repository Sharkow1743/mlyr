#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Source {
    pub kind: String,
    pub name: Option<String>,
    pub content: Option<String>,
}

impl Source {
    pub fn new(kind: impl Into<String>, name: Option<String>, content: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            name,
            content,
        }
    }

    pub fn with_name(kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            name: Some(name.into()),
            content: None,
        }
    }

    pub fn with_content(kind: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            name: None,
            content: Some(content.into()),
        }
    }
}
