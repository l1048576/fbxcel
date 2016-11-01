//! Parser event.

use std::io;
use std::io::Read;

use parser::binary::BinaryParser;
use parser::binary::error::{Result, Error, Warning};

mod attribute;


/// Parser event.
#[derive(Debug)]
pub enum Event<'a, R: 'a + Read> {
    /// Start of the FBX document.
    StartFbx(FbxHeader),
    /// End of the FBX document.
    EndFbx(FbxFooter),
    /// Start of a node.
    StartNode(StartNode<'a, R>),
    /// End of a node.
    EndNode,
}

impl<'a, R: 'a + Read> From<FbxHeader> for Event<'a, R> {
    fn from(h: FbxHeader) -> Self {
        Event::StartFbx(h)
    }
}

impl<'a, R: 'a + Read> From<FbxFooter> for Event<'a, R> {
    fn from(f: FbxFooter) -> Self {
        Event::EndFbx(f)
    }
}

impl<'a, R: 'a + Read> From<StartNode<'a, R>> for Event<'a, R> {
    fn from(h: StartNode<'a, R>) -> Self {
        Event::StartNode(h)
    }
}


/// FBX header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FbxHeader {
    /// FBX version.
    pub version: u32,
}


/// Read FBX header.
pub fn read_fbx_header<R: Read>(parser: &mut BinaryParser<R>) -> Result<FbxHeader> {
    assert!(parser.fbx_version.is_none(),
            "Parser should read FBX header only once");
    // Check magic binary.
    {
        const MAGIC_LEN: usize = 21;
        const MAGIC: &'static [u8; MAGIC_LEN] = b"Kaydara FBX Binary  \x00";
        let mut buf = [0u8; MAGIC_LEN];
        try!(parser.source.read_exact(&mut buf));
        if buf != *MAGIC {
            return Err(Error::MagicNotDetected(buf));
        }
    }
    // Read unknown 2 bytes.
    {
        const UNKNOWN_BYTES_LEN: usize = 2;
        const UNKNOWN_BYTES: &'static [u8; UNKNOWN_BYTES_LEN] = b"\x1a\x00";
        let mut buf = [0u8; UNKNOWN_BYTES_LEN];
        try!(parser.source.read_exact(&mut buf));
        if buf != *UNKNOWN_BYTES {
            parser.warn(Warning::UnexpectedBytesAfterMagic(buf));
        }
    }
    // Get FBX version.
    let fbx_version = try!(parser.source.read_u32());

    info!("FBX header is successfully read, FBX version: {}",
          fbx_version);
    Ok(FbxHeader { version: fbx_version })
}


/// FBX footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FbxFooter {
    /// Unknown part 1.
    pub unknown1: [u8; 16],
    /// FBX version.
    pub version: u32,
    /// Unknown part 2.
    pub unknown2: [u8; 16],
}

impl FbxFooter {
    /// Reads node header from the given parser and returns it.
    pub fn read_from_parser<R: Read>(parser: &mut BinaryParser<R>) -> Result<Self> {
        // Read unknown 16 bytes footer.
        let mut unknown1 = [0u8; 16];
        try!(parser.source.read_exact(&mut unknown1));
        // Read padding (0--15 bytes), zeroes (4 bytes), FBX version (4 bytes), zeroes (120 bytes),
        // and optionally partial unknown footer 2 (16 bytes).
        // Note that some exporters (like Blender's "FBX format" plugin version 3.2.0) creates
        // wrong FBX file without padding.
        // For such file without padding, unknown footer 2 would be partially read.
        let expected_padding_len = ((16 - (parser.source.count() & 0x0f)) & 0x0f) as usize;
        debug!("Current position = {}, Expected padding length = {}",
               parser.source.count(),
               expected_padding_len);

        const BUF_LEN: usize = 144;
        let mut buf = [0u8; BUF_LEN];
        try!(parser.source.read_exact(&mut buf));
        // If there is no padding before the footer, unknown footer 2 is partially read into the
        // buf.
        // Count length of partially read unknown footer 2.
        let partial_footer2_len = {
            let mut count = 0;
            // Unknown footer 2 doesn't contain 0x00 byte, therefore the last 0x00 should be
            // the last byte of a padding.
            while (buf[BUF_LEN - 1 - count] != 0) && count <= 16 {
                count += 1;
            }
            if count > 16 {
                error!("FBX footer should have continuous 112 bytes of zeroes, but not found");
                return Err(Error::BrokenFbxFooter);
            }
            count
        };
        let mut unknown2 = [0u8; 16];
        // Copy partially read unknown header 2.
        unknown2[0..partial_footer2_len]
            .clone_from_slice(&buf[BUF_LEN - partial_footer2_len..BUF_LEN]);
        // Read the rest of the unknown footer 2 (max 16 bytes).
        try!(parser.source.read_exact(&mut unknown2[partial_footer2_len..]));

        // Check whether padding before the footer exists.
        if 16 - partial_footer2_len == expected_padding_len {
            // Padding exists.
            // Note that its length might be 0.
            info!("Padding exists (as expected) before the footer (len={})",
                  expected_padding_len);
        } else if partial_footer2_len == 16 {
            // Padding doesn't exist while it should.
            warn!("Expected padding (len={}) but not found",
                  expected_padding_len);
        } else {
            error!("Unexpected padding length: expected={}, got={}",
                   expected_padding_len,
                   16 - partial_footer2_len);
            return Err(Error::BrokenFbxFooter);
        }

        // Check the FBX version.
        let footer_fbx_version = {
            // 20 - partial_footer2_len == BUF_LEN - partial_footer2_len - 120 - 4
            let ver_offset = 20 - partial_footer2_len;
            // FBX version is stored as `u32` in Little Endian.
            (buf[ver_offset] as u32) | (buf[ver_offset + 1] as u32) << 8 |
            (buf[ver_offset + 2] as u32) << 16 | (buf[ver_offset + 3] as u32) << 24
        };
        let header_fbx_version = parser.fbx_version
            .expect("Parser should remember FBX version in the FBX header but it doesn't");
        if header_fbx_version != footer_fbx_version {
            return Err(Error::HeaderFooterVersionMismatch {
                header: header_fbx_version,
                footer: footer_fbx_version,
            });
        }

        Ok(FbxFooter {
            unknown1: unknown1,
            version: footer_fbx_version,
            unknown2: unknown2,
        })
    }
}

/// FBX node info.
#[derive(Debug)]
pub struct StartNode<'a, R: 'a + Read> {
    /// Node name.
    pub name: String,
    /// Parser.
    _parser: &'a mut BinaryParser<R>,
}


/// Parser event without reference to a parser.
#[derive(Debug, Clone)]
pub enum EventBuilder {
    /// Start of the FBX document.
    StartFbx(FbxHeader),
    /// End of the FBX document.
    EndFbx(FbxFooter),
    /// Start of a node.
    StartNode(StartNodeBuilder),
    /// End of a node.
    EndNode,
}

impl EventBuilder {
    /// Creates `Event` from the `EventBuilder` and the given parser.
    pub fn build<R: Read>(self, parser: &mut BinaryParser<R>) -> Event<R> {
        match self {
            EventBuilder::StartFbx(header) => header.into(),
            EventBuilder::EndFbx(footer) => footer.into(),
            EventBuilder::StartNode(builder) => builder.build(parser).into(),
            EventBuilder::EndNode => Event::EndNode,
        }
    }
}

impl From<FbxHeader> for EventBuilder {
    fn from(h: FbxHeader) -> Self {
        EventBuilder::StartFbx(h)
    }
}

impl From<FbxFooter> for EventBuilder {
    fn from(f: FbxFooter) -> Self {
        EventBuilder::EndFbx(f)
    }
}

impl From<StartNodeBuilder> for EventBuilder {
    fn from(h: StartNodeBuilder) -> Self {
        EventBuilder::StartNode(h)
    }
}


/// `StartNode` without reference to a parser.
#[derive(Debug, Clone)]
pub struct StartNodeBuilder {
    /// Node name.
    pub name: String,
}

impl StartNodeBuilder {
    /// Creates `StartNode` from the `StartNodeBuilder` and the given parser.
    pub fn build<R: Read>(self, parser: &mut BinaryParser<R>) -> StartNode<R> {
        StartNode {
            name: self.name,
            _parser: parser,
        }
    }
}


/// Fixed size node header (without node name field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeHeader {
    /// End offset of the node.
    pub end_offset: u64,
    /// Number of the node attributes.
    pub num_attributes: u64,
    /// Byte length of the node attributes.
    pub len_attributes: u64,
    /// Length of the node name.
    pub len_name: u8,
}

impl NodeHeader {
    /// Returns true if all fields of the node header is `0`.
    pub fn is_node_end(&self) -> bool {
        self.end_offset == 0 && self.num_attributes == 0 && self.len_attributes == 0 &&
        self.len_name == 0
    }

    /// Reads node header from the given parser and returns it.
    pub fn read_from_parser<R: Read>(parser: &mut BinaryParser<R>) -> io::Result<Self> {
        let fbx_version = parser.fbx_version
            .expect("Attempt to read FBX node header but the parser doesn't know FBX version");
        let (end_offset, num_attributes, len_attributes) = if fbx_version < 7500 {
            let eo = try!(parser.source.read_u32()) as u64;
            let na = try!(parser.source.read_u32()) as u64;
            let la = try!(parser.source.read_u32()) as u64;
            (eo, na, la)
        } else {
            let eo = try!(parser.source.read_u64());
            let na = try!(parser.source.read_u64());
            let la = try!(parser.source.read_u64());
            (eo, na, la)
        };
        let len_name = try!(parser.source.read_u8());
        Ok(NodeHeader {
            end_offset: end_offset,
            num_attributes: num_attributes,
            len_attributes: len_attributes,
            len_name: len_name,
        })
    }
}
