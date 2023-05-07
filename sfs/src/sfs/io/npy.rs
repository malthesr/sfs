//! Reading and writing for SFS in the numpy npy format.
//!
//! The npy format is described [here][spec]. Only a subset required to read/write an SFS
//! is supported. Only simple type descriptors for the basic integer and float types are
//! supported. In addition, only reading/writing C-order is supported; trying to read a
//! Fortran-order npy file will result in a run-time error.
//!
//! [spec]: https://numpy.org/neps/nep-0001-npy-format.html

use std::io;

use crate::sfs::{Sfs, Shape};

mod header;
use header::{Endian, Header, HeaderDict, Type, TypeDescriptor, Version};

/// The npy magic number.
pub(crate) const MAGIC: [u8; 6] = *b"\x93NUMPY";

/// Reads an SFS in npy format from a reader.
///
/// The stream is assumed to be positioned at the start.
pub fn read_sfs<R>(reader: &mut R) -> io::Result<Sfs>
where
    R: io::BufRead,
{
    let header = Header::read(reader)?;
    let dict = header.dict;

    match (dict.type_descriptor, dict.fortran_order) {
        (_, true) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Fortran order not supported when reading npy",
        )),
        (descr, false) => {
            let values = descr.read(reader)?;

            Sfs::new(values, Shape(dict.shape)).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "npy shape does not fit values")
            })
        }
    }
}

/// Writes an SFS in npy format to a writer.
pub fn write_sfs<const N: bool, W>(writer: &mut W, sfs: &Sfs<N>) -> io::Result<()>
where
    W: io::Write,
{
    let header = Header::new(
        Version::V1,
        HeaderDict::new(
            TypeDescriptor::new(Endian::Little, Type::F8),
            false,
            sfs.shape().as_ref().to_vec(),
        ),
    );

    header.write(writer)?;

    for v in sfs.iter() {
        writer.write_all(&v.to_le_bytes())?;
    }

    Ok(())
}
