//! Special type node attribute.

use std::io;
use std::io::Read;

use parser::binary::{BinaryParser, CountReader};


/// Attribute type of special value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialAttributeType {
    /// Binary value.
    Binary,
    /// String value.
    ///
    /// Note that the encoding is not specified.
    /// The attribute string may be UTF-8, UTF-16, or any other encodings.
    String,
}


/// Special type attribute.
#[derive(Debug)]
pub struct SpecialAttribute<'a, R: 'a> {
    /// Parser.
    parser: &'a mut BinaryParser<R>,
    /// Value type.
    value_type: SpecialAttributeType,
    /// Length of the value in bytes.
    byte_length: u32,
    /// End offset of the attribute value.
    end_offset: u64,
}

impl<'a, R: 'a + Read> SpecialAttribute<'a, R> {
    /// Returns reader of the raw attribute value.
    pub fn reader(&mut self) -> io::Take<&mut CountReader<R>> {
        let limit = self.rest_len();
        self.parser.source.by_ref().take(limit)
    }

    /// Returns attribute value type.
    pub fn value_type(&self) -> SpecialAttributeType {
        self.value_type
    }

    /// Returns length of the whole value.
    pub fn total_len(&self) -> u64 {
        self.byte_length as u64
    }

    /// Returns rest data size.
    pub fn rest_len(&self) -> u64 {
        self.end_offset - self.parser.source.count()
    }

    /// Read the attribute to the vector.
    pub fn into_vec(mut self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.rest_len() as usize);
        try!(self.reader().read_to_end(&mut buf));
        Ok(buf)
    }

    /// Read the attribute to the string.
    pub fn into_string(mut self) -> io::Result<String> {
        let mut buf = String::with_capacity(self.rest_len() as usize);
        try!(self.reader().read_to_string(&mut buf));
        Ok(buf)
    }
}


/// Read special type attribute from the given parser.
pub fn read_special_attribute<R: Read>(
    parser: &mut BinaryParser<R>,
    type_code: u8
) -> io::Result<(SpecialAttribute<R>, u64)> {
    let byte_length = try!(parser.source.read_u32());
    let value_type = match type_code {
        b'R' => SpecialAttributeType::Binary,
        b'S' => SpecialAttributeType::String,
        _ => unreachable!(),
    };
    let current_pos = parser.source.count();
    let end_offset = current_pos + byte_length as u64;

    Ok((SpecialAttribute {
        parser: parser,
        value_type: value_type,
        byte_length: byte_length,
        end_offset: end_offset,
    },
        end_offset))
}
