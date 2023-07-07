//! Reading and writing in the numpy npy format.
//!
//! The npy format is described [here][spec]. Only a subset required to read/write SFS
//! is supported. Only simple type descriptors for the basic integer and float types are
//! supported. In addition, only reading/writing C-order is supported; trying to read a
//! Fortran-order npy file will result in a run-time error.
//!
//! [spec]: https://numpy.org/neps/nep-0001-npy-format.html

use std::io;

use super::{Array, Shape};

mod header;
use header::{Endian, Header, HeaderDict, Type, TypeDescriptor, Version};

/// The npy magic number.
pub(crate) const MAGIC: [u8; 6] = *b"\x93NUMPY";

/// Reads an array in npy format from a reader.
///
/// The stream is assumed to be positioned at the start.
pub fn read_array<R>(reader: &mut R) -> io::Result<Array<f64>>
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

            Array::new(values, Shape(dict.shape)).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "npy shape does not fit values")
            })
        }
    }
}

/// Writes an array in npy format to a writer.
pub fn write_array<W>(writer: &mut W, array: &Array<f64>) -> io::Result<()>
where
    W: io::Write,
{
    let header = Header::new(
        Version::V1,
        HeaderDict::new(
            TypeDescriptor::new(Endian::Little, Type::F8),
            false,
            array.shape().as_ref().to_vec(),
        ),
    );

    header.write(writer)?;

    for v in array.iter() {
        writer.write_all(&v.to_le_bytes())?;
    }

    Ok(())
}
