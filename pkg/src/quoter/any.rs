use std::{ops::Deref, sync::Arc};

use crate::quoter::Quoter;

#[derive(Debug, Clone)]
pub struct AnyQuoter(pub Arc<dyn Quoter>);

impl<T> From<T> for AnyQuoter
where
    T: Quoter + 'static,
{
    fn from(t: T) -> Self {
        AnyQuoter(Arc::new(t))
    }
}

impl Deref for AnyQuoter {
    type Target = Arc<dyn Quoter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
