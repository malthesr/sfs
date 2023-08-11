use crate::input::{ReadStatus, Sample};

pub mod bcf;

pub mod vcf;

use super::Result;

pub trait Reader {
    fn current_contig(&self) -> &str;

    fn current_position(&self) -> usize;

    fn read_genotypes(&mut self) -> ReadStatus<Vec<Result>>;

    fn samples(&self) -> &[Sample];
}
