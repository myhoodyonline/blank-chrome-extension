//! AVM1 object type to represent the attributes of XML nodes

use crate::avm1::activation::Activation;
use crate::avm1::error::Error;
use crate::avm1::object::{ObjectPtr, TObject};
use crate::avm1::property::Attribute;
use crate::avm1::{Object, ScriptObject, Value};
use crate::avm_warn;
use crate::xml::{XmlDocument, XmlNode};
use gc_arena::{Collect, MutationContext};
use std::borrow::Cow;
use std::fmt;

/// An Object that is inherently tied to an XML document's ID map.
///
/// Note that this is *not* the same as the XML root object itself; and
/// furthermore this object is linked to the document, not the root node of the
/// document.
#[derive(Clone, Copy, Collect)]
#[collect(no_drop)]
pub struct XmlIdMapObject<'gc>(ScriptObject<'gc>, XmlDocument<'gc>);

impl<'gc> XmlIdMapObject<'gc> {
    /// Construct an XmlIdMapObject for an already existing node's
    /// attributes.
    pub fn from_xml_document(
        gc_context: MutationContext<'gc, '_>,
        xml_doc: XmlDocument<'gc>,
    ) -> Object<'gc> {
        XmlIdMapObject(ScriptObject::object(gc_context, None), xml_doc).into()
    }

    fn base(&self) -> ScriptObject<'gc> {
        match self {
            XmlIdMapObject(base, ..) => *base,
        }
    }

    fn document(&self) -> XmlDocument<'gc> {
        match self {
            XmlIdMapObject(_, document) => *document,
        }
    }
}

impl fmt::Debug for XmlIdMapObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            XmlIdMapObject(base, document) => f
                .debug_tuple("XmlIdMapObject")
                .field(base)
                .field(document)
                .finish(),
        }
    }
}

impl<'gc> TObject<'gc> for XmlIdMapObject<'gc> {
    fn get_local(
        &self,
        name: &str,
        activation: &mut Activation<'_, 'gc, '_>,
        this: Object<'gc>,
    ) -> Result<Value<'gc>, Error<'gc>> {
        if let Some(mut node) = self.document().get_node_by_id(name) {
            Ok(node
                .script_object(
                    activation.context.gc_context,
                    Some(activation.context.avm1.prototypes().xml_node),
                )
                .into())
        } else {
            self.base().get_local(name, activation, this)
        }
    }

    fn set(
        &self,
        name: &str,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error<'gc>> {
        self.base().set(name, value, activation)
    }
    fn call(
        &self,
        name: &str,
        activation: &mut Activation<'_, 'gc, '_>,
        this: Object<'gc>,
        base_proto: Option<Object<'gc>>,
        args: &[Value<'gc>],
    ) -> Result<Value<'gc>, Error<'gc>> {
        self.base().call(name, activation, this, base_proto, args)
    }

    fn call_setter(
        &self,
        name: &str,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Option<Object<'gc>> {
        self.base().call_setter(name, value, activation)
    }

    #[allow(clippy::new_ret_no_self)]
    fn create_bare_object(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        _this: Object<'gc>,
    ) -> Result<Object<'gc>, Error<'gc>> {
        //TODO: `new xmlnode.attributes()` returns undefined, not an object
        avm_warn!(activation, "Cannot create new XML Attributes object");
        Ok(Value::Undefined.coerce_to_object(activation))
    }

    fn delete(&self, activation: &mut Activation<'_, 'gc, '_>, name: &str) -> bool {
        self.base().delete(activation, name)
    }

    fn add_property(
        &self,
        gc_context: MutationContext<'gc, '_>,
        name: &str,
        get: Object<'gc>,
        set: Option<Object<'gc>>,
        attributes: Attribute,
    ) {
        self.base()
            .add_property(gc_context, name, get, set, attributes)
    }

    fn add_property_with_case(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        gc_context: MutationContext<'gc, '_>,
        name: &str,
        get: Object<'gc>,
        set: Option<Object<'gc>>,
        attributes: Attribute,
    ) {
        self.base()
            .add_property_with_case(activation, gc_context, name, get, set, attributes)
    }

    fn set_watcher(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        gc_context: MutationContext<'gc, '_>,
        name: Cow<str>,
        callback: Object<'gc>,
        user_data: Value<'gc>,
    ) {
        self.base()
            .set_watcher(activation, gc_context, name, callback, user_data);
    }

    fn remove_watcher(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        gc_context: MutationContext<'gc, '_>,
        name: Cow<str>,
    ) -> bool {
        self.base().remove_watcher(activation, gc_context, name)
    }

    fn define_value(
        &self,
        gc_context: MutationContext<'gc, '_>,
        name: &str,
        value: Value<'gc>,
        attributes: Attribute,
    ) {
        self.base()
            .define_value(gc_context, name, value, attributes)
    }

    fn set_attributes(
        &self,
        gc_context: MutationContext<'gc, '_>,
        name: Option<&str>,
        set_attributes: Attribute,
        clear_attributes: Attribute,
    ) {
        self.base()
            .set_attributes(gc_context, name, set_attributes, clear_attributes)
    }

    fn proto(&self) -> Option<Object<'gc>> {
        self.base().proto()
    }

    fn set_proto(&self, gc_context: MutationContext<'gc, '_>, prototype: Option<Object<'gc>>) {
        self.base().set_proto(gc_context, prototype);
    }

    fn has_property(&self, activation: &mut Activation<'_, 'gc, '_>, name: &str) -> bool {
        self.base().has_property(activation, name)
    }

    fn has_own_property(&self, activation: &mut Activation<'_, 'gc, '_>, name: &str) -> bool {
        self.document().get_node_by_id(name).is_some()
            || self.base().has_own_property(activation, name)
    }

    fn has_own_virtual(&self, activation: &mut Activation<'_, 'gc, '_>, name: &str) -> bool {
        self.base().has_own_virtual(activation, name)
    }

    fn is_property_enumerable(&self, activation: &mut Activation<'_, 'gc, '_>, name: &str) -> bool {
        self.base().is_property_enumerable(activation, name)
    }

    fn get_keys(&self, activation: &mut Activation<'_, 'gc, '_>) -> Vec<String> {
        let mut keys = self.base().get_keys(activation);
        keys.extend(self.document().get_node_ids().into_iter());
        keys
    }

    fn as_string(&self) -> Cow<str> {
        Cow::Owned(self.base().as_string().into_owned())
    }

    fn type_of(&self) -> &'static str {
        self.base().type_of()
    }

    fn interfaces(&self) -> Vec<Object<'gc>> {
        self.base().interfaces()
    }

    fn set_interfaces(&self, gc_context: MutationContext<'gc, '_>, iface_list: Vec<Object<'gc>>) {
        self.base().set_interfaces(gc_context, iface_list)
    }

    fn as_script_object(&self) -> Option<ScriptObject<'gc>> {
        Some(self.base())
    }

    fn as_xml_node(&self) -> Option<XmlNode<'gc>> {
        None
    }

    fn as_ptr(&self) -> *const ObjectPtr {
        self.base().as_ptr() as *const ObjectPtr
    }

    fn length(&self) -> usize {
        self.base().length()
    }

    fn array(&self) -> Vec<Value<'gc>> {
        self.base().array()
    }

    fn set_length(&self, gc_context: MutationContext<'gc, '_>, length: usize) {
        self.base().set_length(gc_context, length)
    }

    fn array_element(&self, index: usize) -> Value<'gc> {
        self.base().array_element(index)
    }

    fn set_array_element(
        &self,
        index: usize,
        value: Value<'gc>,
        gc_context: MutationContext<'gc, '_>,
    ) -> usize {
        self.base().set_array_element(index, value, gc_context)
    }

    fn delete_array_element(&self, index: usize, gc_context: MutationContext<'gc, '_>) {
        self.base().delete_array_element(index, gc_context)
    }
}
