use std::io;

use bcf::lazy::Record as BcfRecord;
use noodles_bcf as bcf;

use noodles_vcf as vcf;
use vcf::record::genotypes::sample::value::genotype::Genotype as VcfGenotype;

use crate::reader::{genotype, ReadStatus, Sample};

pub struct Reader<R> {
    pub inner: bcf::Reader<R>,
    pub header: vcf::Header,
    pub string_maps: bcf::header::StringMaps,
    pub samples: Vec<Sample>,
    pub buf: BcfRecord,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> io::Result<Self> {
        let mut inner = bcf::Reader::from(inner);

        let header = inner.read_header()?;
        let string_maps = bcf::header::StringMaps::try_from(&header)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let samples = header
            .sample_names()
            .iter()
            .cloned()
            .map(Sample::from)
            .collect();

        Ok(Self {
            inner,
            header,
            string_maps,
            samples,
            buf: BcfRecord::default(),
        })
    }

    fn read_genotypes(&mut self) -> ReadStatus<Vec<Option<VcfGenotype>>> {
        match self.inner.read_lazy_record(&mut self.buf) {
            Ok(0) => ReadStatus::Done,
            Ok(_) => {
                let result = self
                    .buf
                    .genotypes()
                    .try_into_vcf_record_genotypes(&self.header, self.string_maps.strings())
                    .and_then(|genotypes| {
                        genotypes
                            .genotypes()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                    });

                match result {
                    Ok(genotypes) => ReadStatus::Read(genotypes),
                    Err(e) => ReadStatus::Error(e),
                }
            }
            Err(e) => ReadStatus::Error(e),
        }
    }
}

impl<R> super::Reader for Reader<R>
where
    R: io::Read,
{
    fn current_contig(&self) -> &str {
        self.string_maps
            .contigs()
            .get_index(self.buf.chromosome_id())
            .unwrap_or("[unknown]")
    }

    fn current_position(&self) -> usize {
        self.buf.position().into()
    }

    fn read_genotypes(&mut self) -> ReadStatus<Vec<genotype::Result>> {
        self.read_genotypes().map(|vcf_genotypes| {
            vcf_genotypes
                .into_iter()
                .map(genotype::Result::from)
                .collect()
        })
    }

    fn samples(&self) -> &[Sample] {
        &self.samples
    }
}
