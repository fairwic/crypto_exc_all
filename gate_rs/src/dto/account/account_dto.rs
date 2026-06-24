#[derive(Debug, Clone, Default)]
pub struct GateAccountBookRequest {
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u32>,
    pub book_type: Option<String>,
}

impl GateAccountBookRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_from(mut self, value: u64) -> Self {
        self.from = Some(value);
        self
    }

    pub fn with_to(mut self, value: u64) -> Self {
        self.to = Some(value);
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    pub fn with_type(mut self, value: impl Into<String>) -> Self {
        self.book_type = Some(value.into());
        self
    }
}
