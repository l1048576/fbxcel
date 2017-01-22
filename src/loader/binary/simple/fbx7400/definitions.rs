//! `Definitions` node and its children.

use fnv::FnvHashMap;
use parser::binary::{Parser, ParserSource, Event, Attributes};
use loader::binary::simple::{Result, Error};
use loader::binary::simple::fbx7400::Properties70;


/// `Definitions` node.
#[derive(Debug, Clone, PartialEq)]
pub struct Definitions {
    /// Version of the node.
    pub version: i32,
    /// Reference count?
    pub count: i32,
    /// Property templates for object types.
    pub object_types: Vec<ObjectType>,
}

impl Definitions {
    /// Loads node contents from the parser.
    pub fn load<R: ParserSource, P: Parser<R>>(mut parser: P) -> Result<Self> {
        let mut version = None;
        let mut count = None;
        let mut object_types = Vec::new();

        loop {
            let node_type = match parser.next_event()? {
                Event::StartFbx(_) |
                Event::EndFbx(_) => unreachable!(),
                Event::StartNode(info) => DefinitionsChildAttrs::load(info.name, info.attributes)?,
                Event::EndNode => break,
            };
            match node_type {
                DefinitionsChildAttrs::Version(v) => {
                    version = Some(v);
                    parser.skip_current_node()?;
                },
                DefinitionsChildAttrs::Count(v) => {
                    count = Some(v);
                    parser.skip_current_node()?;
                },
                DefinitionsChildAttrs::ObjectType(attrs) => {
                    object_types.push(ObjectType::load(parser.subtree_parser(), attrs)?);
                },
            }
        }
        Ok(Definitions {
            version: version.ok_or_else(|| Error::MissingNode("Definitions".to_owned()))?,
            count: count.ok_or_else(|| Error::MissingNode("Definitions".to_owned()))?,
            object_types: object_types,
        })
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DefinitionsChildAttrs {
    Version(i32),
    Count(i32),
    ObjectType(String),
}

impl DefinitionsChildAttrs {
    /// Creates a `DefinitionsChildAttrs` from the given node name.
    pub fn load<R: ParserSource>(name: &str, mut attrs: Attributes<R>) -> Result<Self> {
        use parser::binary::utils::AttributeValues;

        match name {
            "Version" => {
                <i32>::from_attributes(&mut attrs)
                    ?
                    .ok_or_else(|| Error::InvalidAttribute(name.to_owned()))
                    .map(DefinitionsChildAttrs::Version)
            },
            "Count" => {
                <i32>::from_attributes(&mut attrs)
                    ?
                    .ok_or_else(|| Error::InvalidAttribute(name.to_owned()))
                    .map(DefinitionsChildAttrs::Count)
            },
            "ObjectType" => {
                <String>::from_attributes(&mut attrs)
                    ?
                    .ok_or_else(|| Error::InvalidAttribute(name.to_owned()))
                    .map(DefinitionsChildAttrs::ObjectType)
            },
            _ => Err(Error::UnexpectedNode(name.to_owned())),
        }
    }
}


/// An object type and property template for it.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectType {
    /// Target object type.
    pub object_type: String,
    /// Reference count?
    pub count: i32,
    /// Property templates.
    pub property_template: FnvHashMap<String, Properties70>,
}

impl ObjectType {
    /// Loads node contents from the parser.
    pub fn load<R: ParserSource, P: Parser<R>>(mut parser: P, attrs: String) -> Result<Self> {
        let mut count = None;
        let mut property_template = FnvHashMap::default();

        loop {
            let node_type = match parser.next_event()? {
                Event::StartFbx(_) |
                Event::EndFbx(_) => unreachable!(),
                Event::StartNode(info) => ObjectTypeChildAttrs::load(info.name, info.attributes)?,
                Event::EndNode => break,
            };
            match node_type {
                ObjectTypeChildAttrs::Count(v) => {
                    count = Some(v);
                    parser.skip_current_node()?;
                },
                ObjectTypeChildAttrs::PropertyTemplate(attrs) => {
                    let props = load_property_template(parser.subtree_parser())?;
                    property_template.insert(attrs, props);
                },
            }
        }

        Ok(ObjectType {
            object_type: attrs,
            count: count.ok_or_else(|| Error::MissingNode("ObjectType".to_owned()))?,
            property_template: property_template,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ObjectTypeChildAttrs {
    Count(i32),
    PropertyTemplate(String),
}

impl ObjectTypeChildAttrs {
    /// Creates an `ObjectTypeChildAttrs` from the given node name.
    pub fn load<R: ParserSource>(name: &str, mut attrs: Attributes<R>) -> Result<Self> {
        use parser::binary::utils::AttributeValues;

        match name {
            "Count" => {
                <i32>::from_attributes(&mut attrs)
                    ?
                    .ok_or_else(|| Error::InvalidAttribute(name.to_owned()))
                    .map(ObjectTypeChildAttrs::Count)
            },
            "PropertyTemplate" => {
                <String>::from_attributes(&mut attrs)
                    ?
                    .ok_or_else(|| Error::InvalidAttribute(name.to_owned()))
                    .map(ObjectTypeChildAttrs::PropertyTemplate)
            },
            _ => Err(Error::UnexpectedNode(name.to_owned())),
        }
    }
}


fn load_property_template<R: ParserSource, P: Parser<R>>(mut parser: P) -> Result<Properties70> {
    let mut props = None;

    loop {
        match parser.next_event()? {
            Event::StartFbx(_) |
            Event::EndFbx(_) => unreachable!(),
            Event::StartNode(info) => {
                if info.name != "Properties70" {
                    return Err(Error::UnexpectedNode(info.name.to_owned()));
                }
            },
            Event::EndNode => break,
        }
        props = Some(Properties70::load(parser.subtree_parser())?);
    }
    Ok(props.ok_or_else(|| Error::MissingNode("PropertyTemplate".to_owned()))?)
}
