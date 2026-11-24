#[derive(Debug, PartialEq, Clone, Default)]
pub struct Sources {
    pub list: Vec<Source>,
}

impl Sources {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Source {
    pub name: String,
    pub url: Option<String>,
}
