use std::fmt::{self, Display};

/// The direction to quote along a quoter edge.
///
/// `Forward` means `token0 -> token1` for the pair returned by [`Quoter::get_tokens`].
/// `Reverse` means the inverse direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateDirection {
    Forward,
    Reverse,
}

impl Display for RateDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
