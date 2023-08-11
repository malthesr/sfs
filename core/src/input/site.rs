pub mod reader;
pub use reader::Reader;

use crate::spectrum::project::Projected;

pub enum Site<'a> {
    Standard(&'a [usize]),
    Projected(Projected<'a>),
    InsufficientData,
}
