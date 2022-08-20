//! AVM1 object type to represent XML nodes

use crate::avm1::activation::Activation;
use crate::avm1::error::Error;
use crate::avm1::object::TObject;
use crate::avm1::{Object, ScriptObject};
use crate::impl_custom_object;
use crate::xml::{XmlDocument, XmlNode};
use gc_arena::{Collect, GcCell, MutationContext};
use std::fmt;

/// A ScriptObject that is inherently tied to an XML node.
#[derive(Clone, Copy, Collect)]
#[collect(no_drop)]
pub struct XmlObject<'gc>(GcCell<'gc, XmlObjectData<'gc>>);

#[derive(Clone, Collect)]
#[collect(no_drop)]
pub struct XmlObjectData<'gc> {
    base: ScriptObject<'gc>,
    node: XmlNode<'gc>,
}

impl<'gc> XmlObject<'gc> {
    /// Construct a new XML node and object pair.
    pub fn empty_node(
        gc_context: MutationContext<'gc, '_>,
        proto: Option<Object<'gc>>,
    ) -> Object<'gc> {
        let empty_document = XmlDocument::new(gc_context);
        let mut xml_node = XmlNode::new_text(gc_context, "", empty_document);
        let base_object = ScriptObject::object(gc_context, proto);
        let object = XmlObject(GcCell::allocate(
            gc_context,
            XmlObjectData {
                base: base_object,
                node: xml_node,
            },
        ))
        .into();

        xml_node.introduce_script_object(gc_context, object);

        object
    }

    /// Construct an XmlObject for an already existing node.
    pub fn from_xml_node(
        gc_context: MutationContext<'gc, '_>,
        xml_node: XmlNode<'gc>,
        proto: Option<Object<'gc>>,
    ) -> Object<'gc> {
        XmlObject(GcCell::allocate(
            gc_context,
            XmlObjectData {
                base: ScriptObject::object(gc_context, proto),
                node: xml_node,
            },
        ))
        .into()
    }
}

impl fmt::Debug for XmlObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let this = self.0.read();
        f.debug_struct("XmlObject")
            .field("base", &this.base)
            .field("node", &this.node)
            .finish()
    }
}

impl<'gc> TObject<'gc> for XmlObject<'gc> {
    impl_custom_object!(base);

    #[allow(clippy::new_ret_no_self)]
    fn create_bare_object(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        this: Object<'gc>,
    ) -> Result<Object<'gc>, Error<'gc>> {
        Ok(XmlObject::empty_node(
            activation.context.gc_context,
            Some(this),
        ))
    }

    fn as_xml_node(&self) -> Option<XmlNode<'gc>> {
        Some(self.0.read().node)
    }
}
