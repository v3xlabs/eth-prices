use std::{ops::Deref, sync::Arc};

use crate::quoter::Quoter;

#[derive(Debug, Clone)]
pub struct AnyQuoter {
    quoter: Arc<dyn Quoter>,
    pub confidence: u64,
}

impl AnyQuoter {
    pub fn with_confidence(mut self, confidence: u64) -> Self {
        self.confidence = confidence;
        self
    }
}

impl<T> From<T> for AnyQuoter
where
    T: Quoter + 'static,
{
    fn from(t: T) -> Self {
        AnyQuoter {
            quoter: Arc::new(t),
            confidence: 0,
        }
    }
}

impl Deref for AnyQuoter {
    type Target = dyn Quoter;

    fn deref(&self) -> &Self::Target {
        &*self.quoter
    }
}
