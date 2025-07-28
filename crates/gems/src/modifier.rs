#[derive(Debug, Clone, Copy)]
pub struct Modifier {
    value: f64,
    kind: ModifierKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ModifierKind {
    Additive,
    More,
}

impl Modifier {
    pub fn new_add(value: f64) -> Self {
        Self {
            value,
            kind: ModifierKind::Additive,
        }
    }

    pub fn new_more(value: f64) -> Self {
        Self {
            value,
            kind: ModifierKind::More,
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn kind(&self) -> ModifierKind {
        self.kind
    }

    pub fn factor(&self) -> f64 {
        1. + self.value
    }
}
