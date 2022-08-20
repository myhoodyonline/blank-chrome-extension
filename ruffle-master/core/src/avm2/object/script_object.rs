//! Default AVM2 object impl

use crate::avm2::activation::Activation;
use crate::avm2::class::Class;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::{Object, ObjectPtr, TObject};
use crate::avm2::property::Property;
use crate::avm2::property_map::PropertyMap;
use crate::avm2::return_value::ReturnValue;
use crate::avm2::scope::Scope;
use crate::avm2::slot::Slot;
use crate::avm2::string::AvmString;
use crate::avm2::traits::Trait;
use crate::avm2::value::Value;
use crate::avm2::Error;
use gc_arena::{Collect, GcCell, MutationContext};
use std::collections::HashMap;
use std::fmt::Debug;

/// Default implementation of `avm2::Object`.
#[derive(Clone, Collect, Debug, Copy)]
#[collect(no_drop)]
pub struct ScriptObject<'gc>(GcCell<'gc, ScriptObjectData<'gc>>);

/// Information necessary for a script object to have a class attached to it.
///
/// Classes can be attached to a `ScriptObject` such that the class's traits
/// are instantiated on-demand. Either class or instance traits can be
/// instantiated.
///
/// Trait instantiation obeys prototyping rules: prototypes provide their
/// instances with classes to pull traits from.
#[derive(Clone, Collect, Debug)]
#[collect(no_drop)]
pub enum ScriptObjectClass<'gc> {
    /// Instantiate instance traits, for prototypes.
    InstancePrototype(GcCell<'gc, Class<'gc>>, Option<GcCell<'gc, Scope<'gc>>>),

    /// Instantiate class traits, for class constructors.
    ClassConstructor(GcCell<'gc, Class<'gc>>, Option<GcCell<'gc, Scope<'gc>>>),

    /// Do not instantiate any class or instance traits.
    NoClass,
}

/// Base data common to all `TObject` implementations.
///
/// Host implementations of `TObject` should embed `ScriptObjectData` and
/// forward any trait method implementations it does not overwrite to this
/// struct.
#[derive(Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct ScriptObjectData<'gc> {
    /// Properties stored on this object.
    values: PropertyMap<'gc, Property<'gc>>,

    /// Slots stored on this object.
    slots: Vec<Slot<'gc>>,

    /// Methods stored on this object.
    methods: Vec<Option<Object<'gc>>>,

    /// Implicit prototype of this script object.
    proto: Option<Object<'gc>>,

    /// The class that this script object represents.
    class: ScriptObjectClass<'gc>,

    /// Enumeratable property names.
    enumerants: Vec<QName<'gc>>,

    /// Interfaces implemented by this object. (prototypes only)
    interfaces: Vec<Object<'gc>>,
}

impl<'gc> TObject<'gc> for ScriptObject<'gc> {
    fn get_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<Value<'gc>, Error> {
        let rv = self
            .0
            .read()
            .get_property_local(receiver, name, activation)?;

        rv.resolve(activation)
    }

    fn set_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        let rv = self
            .0
            .write(activation.context.gc_context)
            .set_property_local(receiver, name, value, activation)?;

        rv.resolve(activation)?;

        Ok(())
    }

    fn init_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        let rv = self
            .0
            .write(activation.context.gc_context)
            .init_property_local(receiver, name, value, activation)?;

        rv.resolve(activation)?;

        Ok(())
    }

    fn is_property_overwritable(
        self,
        gc_context: MutationContext<'gc, '_>,
        name: &QName<'gc>,
    ) -> bool {
        self.0.write(gc_context).is_property_overwritable(name)
    }

    fn delete_property(&self, gc_context: MutationContext<'gc, '_>, name: &QName<'gc>) -> bool {
        self.0.write(gc_context).delete_property(name)
    }

    fn get_slot(self, id: u32) -> Result<Value<'gc>, Error> {
        self.0.read().get_slot(id)
    }

    fn set_slot(
        self,
        id: u32,
        value: Value<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error> {
        self.0.write(mc).set_slot(id, value, mc)
    }

    fn init_slot(
        self,
        id: u32,
        value: Value<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error> {
        self.0.write(mc).init_slot(id, value, mc)
    }

    fn get_method(self, id: u32) -> Option<Object<'gc>> {
        self.0.read().get_method(id)
    }

    fn get_trait(self, name: &QName<'gc>) -> Result<Vec<Trait<'gc>>, Error> {
        self.0.read().get_trait(name)
    }

    fn get_provided_trait(
        &self,
        name: &QName<'gc>,
        known_traits: &mut Vec<Trait<'gc>>,
    ) -> Result<(), Error> {
        self.0.read().get_provided_trait(name, known_traits)
    }

    fn get_scope(self) -> Option<GcCell<'gc, Scope<'gc>>> {
        self.0.read().get_scope()
    }

    fn resolve_any(self, local_name: AvmString<'gc>) -> Result<Option<Namespace<'gc>>, Error> {
        self.0.read().resolve_any(local_name)
    }

    fn resolve_any_trait(
        self,
        local_name: AvmString<'gc>,
    ) -> Result<Option<Namespace<'gc>>, Error> {
        self.0.read().resolve_any_trait(local_name)
    }

    fn has_own_property(self, name: &QName<'gc>) -> Result<bool, Error> {
        self.0.read().has_own_property(name)
    }

    fn has_trait(self, name: &QName<'gc>) -> Result<bool, Error> {
        self.0.read().has_trait(name)
    }

    fn provides_trait(self, name: &QName<'gc>) -> Result<bool, Error> {
        self.0.read().provides_trait(name)
    }

    fn has_instantiated_property(self, name: &QName<'gc>) -> bool {
        self.0.read().has_instantiated_property(name)
    }

    fn has_own_virtual_getter(self, name: &QName<'gc>) -> bool {
        self.0.read().has_own_virtual_getter(name)
    }

    fn has_own_virtual_setter(self, name: &QName<'gc>) -> bool {
        self.0.read().has_own_virtual_setter(name)
    }

    fn proto(&self) -> Option<Object<'gc>> {
        self.0.read().proto
    }

    fn set_proto(self, mc: MutationContext<'gc, '_>, proto: Object<'gc>) {
        self.0.write(mc).set_proto(proto)
    }

    fn get_enumerant_name(&self, index: u32) -> Option<QName<'gc>> {
        self.0.read().get_enumerant_name(index)
    }

    fn property_is_enumerable(&self, name: &QName<'gc>) -> bool {
        self.0.read().property_is_enumerable(name)
    }

    fn set_local_property_is_enumerable(
        &self,
        mc: MutationContext<'gc, '_>,
        name: &QName<'gc>,
        is_enumerable: bool,
    ) -> Result<(), Error> {
        self.0
            .write(mc)
            .set_local_property_is_enumerable(name, is_enumerable)
    }

    fn as_ptr(&self) -> *const ObjectPtr {
        self.0.as_ptr() as *const ObjectPtr
    }

    fn construct(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        _args: &[Value<'gc>],
    ) -> Result<Object<'gc>, Error> {
        let this: Object<'gc> = Object::ScriptObject(*self);
        Ok(ScriptObject::object(activation.context.gc_context, this))
    }

    fn derive(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Result<Object<'gc>, Error> {
        let this: Object<'gc> = Object::ScriptObject(*self);
        Ok(ScriptObject::prototype(
            activation.context.gc_context,
            this,
            class,
            scope,
        ))
    }

    fn to_string(&self, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        if let Some(class) = self.as_proto_class() {
            Ok(AvmString::new(mc, format!("[object {}]", class.read().name().local_name())).into())
        } else {
            Ok("[object Object]".into())
        }
    }

    fn value_of(&self, _mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        Ok(Value::Object(Object::from(*self)))
    }

    fn install_method(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) {
        self.0.write(mc).install_method(name, disp_id, function)
    }

    fn install_getter(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error> {
        self.0.write(mc).install_getter(name, disp_id, function)
    }

    fn install_setter(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error> {
        self.0.write(mc).install_setter(name, disp_id, function)
    }

    fn install_dynamic_property(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        value: Value<'gc>,
    ) -> Result<(), Error> {
        self.0.write(mc).install_dynamic_property(name, value)
    }

    fn install_slot(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        id: u32,
        value: Value<'gc>,
    ) {
        self.0.write(mc).install_slot(name, id, value)
    }

    fn install_const(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        id: u32,
        value: Value<'gc>,
    ) {
        self.0.write(mc).install_const(name, id, value)
    }

    fn interfaces(&self) -> Vec<Object<'gc>> {
        self.0.read().interfaces()
    }

    fn set_interfaces(&self, gc_context: MutationContext<'gc, '_>, iface_list: Vec<Object<'gc>>) {
        self.0.write(gc_context).set_interfaces(iface_list)
    }

    fn as_class(&self) -> Option<GcCell<'gc, Class<'gc>>> {
        self.0.read().as_class()
    }
}

impl<'gc> ScriptObject<'gc> {
    /// Construct a bare object with no base class.
    ///
    /// This is *not* the same thing as an object literal, which actually does
    /// have a base class: `Object`.
    pub fn bare_object(mc: MutationContext<'gc, '_>) -> Object<'gc> {
        ScriptObject(GcCell::allocate(
            mc,
            ScriptObjectData::base_new(None, ScriptObjectClass::NoClass),
        ))
        .into()
    }

    /// Construct a bare class prototype with no base class.
    ///
    /// This appears to be used specifically for interfaces, which have no base
    /// class.
    pub fn bare_prototype(
        mc: MutationContext<'gc, '_>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Object<'gc> {
        let script_class = ScriptObjectClass::InstancePrototype(class, scope);

        ScriptObject(GcCell::allocate(
            mc,
            ScriptObjectData::base_new(None, script_class),
        ))
        .into()
    }

    /// Construct an object with a prototype.
    pub fn object(mc: MutationContext<'gc, '_>, proto: Object<'gc>) -> Object<'gc> {
        ScriptObject(GcCell::allocate(
            mc,
            ScriptObjectData::base_new(Some(proto), ScriptObjectClass::NoClass),
        ))
        .into()
    }

    /// Construct a prototype for an ES4 class.
    pub fn prototype(
        mc: MutationContext<'gc, '_>,
        proto: Object<'gc>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Object<'gc> {
        let script_class = ScriptObjectClass::InstancePrototype(class, scope);

        ScriptObject(GcCell::allocate(
            mc,
            ScriptObjectData::base_new(Some(proto), script_class),
        ))
        .into()
    }
}

impl<'gc> ScriptObjectData<'gc> {
    pub fn base_new(proto: Option<Object<'gc>>, trait_source: ScriptObjectClass<'gc>) -> Self {
        ScriptObjectData {
            values: HashMap::new(),
            slots: Vec::new(),
            methods: Vec::new(),
            proto,
            class: trait_source,
            enumerants: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    pub fn get_property_local(
        &self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<ReturnValue<'gc>, Error> {
        let prop = self.values.get(name);

        if let Some(prop) = prop {
            prop.get(receiver, activation.base_proto().or(self.proto))
        } else {
            Ok(Value::Undefined.into())
        }
    }

    pub fn set_property_local(
        &mut self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<ReturnValue<'gc>, Error> {
        let slot_id = if let Some(prop) = self.values.get(name) {
            if let Some(slot_id) = prop.slot_id() {
                Some(slot_id)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(slot_id) = slot_id {
            self.set_slot(slot_id, value, activation.context.gc_context)?;
            Ok(Value::Undefined.into())
        } else if self.values.contains_key(name) {
            let prop = self.values.get_mut(name).unwrap();
            let proto = self.proto;
            prop.set(receiver, activation.base_proto().or(proto), value)
        } else {
            //TODO: Not all classes are dynamic like this
            self.enumerants.push(name.clone());
            self.values
                .insert(name.clone(), Property::new_dynamic_property(value));

            Ok(Value::Undefined.into())
        }
    }

    pub fn init_property_local(
        &mut self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<ReturnValue<'gc>, Error> {
        if let Some(prop) = self.values.get_mut(name) {
            if let Some(slot_id) = prop.slot_id() {
                self.init_slot(slot_id, value, activation.context.gc_context)?;
                Ok(Value::Undefined.into())
            } else {
                let proto = self.proto;
                prop.init(receiver, activation.base_proto().or(proto), value)
            }
        } else {
            //TODO: Not all classes are dynamic like this
            self.values
                .insert(name.clone(), Property::new_dynamic_property(value));

            Ok(Value::Undefined.into())
        }
    }

    pub fn is_property_overwritable(&self, name: &QName<'gc>) -> bool {
        self.values
            .get(name)
            .map(|p| p.is_overwritable())
            .unwrap_or(true)
    }

    pub fn delete_property(&mut self, name: &QName<'gc>) -> bool {
        let can_delete = if let Some(prop) = self.values.get(name) {
            prop.can_delete()
        } else {
            false
        };

        if can_delete {
            self.values.remove(name);
        }

        can_delete
    }

    pub fn get_slot(&self, id: u32) -> Result<Value<'gc>, Error> {
        //TODO: slot inheritance, I think?
        self.slots
            .get(id as usize)
            .cloned()
            .ok_or_else(|| format!("Slot index {} out of bounds!", id).into())
            .map(|slot| slot.get().unwrap_or(Value::Undefined))
    }

    /// Set a slot by its index.
    pub fn set_slot(
        &mut self,
        id: u32,
        value: Value<'gc>,
        _mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error> {
        if let Some(slot) = self.slots.get_mut(id as usize) {
            slot.set(value)
        } else {
            Err(format!("Slot index {} out of bounds!", id).into())
        }
    }

    /// Set a slot by its index.
    pub fn init_slot(
        &mut self,
        id: u32,
        value: Value<'gc>,
        _mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error> {
        if let Some(slot) = self.slots.get_mut(id as usize) {
            slot.init(value)
        } else {
            Err(format!("Slot index {} out of bounds!", id).into())
        }
    }

    /// Retrieve a method from the method table.
    pub fn get_method(&self, id: u32) -> Option<Object<'gc>> {
        self.methods.get(id as usize).and_then(|v| *v)
    }

    pub fn get_trait(&self, name: &QName<'gc>) -> Result<Vec<Trait<'gc>>, Error> {
        match &self.class {
            //Class constructors have local traits only.
            ScriptObjectClass::ClassConstructor(..) => {
                let mut known_traits = Vec::new();
                self.get_provided_trait(name, &mut known_traits)?;

                Ok(known_traits)
            }

            //Prototypes do not have traits available locally, but they provide
            //traits instead.
            ScriptObjectClass::InstancePrototype(..) => Ok(Vec::new()),

            //Instances walk the prototype chain to build a list of known
            //traits provided by the classes attached to those prototypes.
            ScriptObjectClass::NoClass => {
                let mut known_traits = Vec::new();
                let mut chain = Vec::new();
                let mut proto = self.proto();

                while let Some(p) = proto {
                    chain.push(p);
                    proto = p.proto();
                }

                for proto in chain.iter().rev() {
                    proto.get_provided_trait(name, &mut known_traits)?;
                }

                Ok(known_traits)
            }
        }
    }

    pub fn get_provided_trait(
        &self,
        name: &QName<'gc>,
        known_traits: &mut Vec<Trait<'gc>>,
    ) -> Result<(), Error> {
        match &self.class {
            ScriptObjectClass::ClassConstructor(class, ..) => {
                class.read().lookup_class_traits(name, known_traits)
            }
            ScriptObjectClass::InstancePrototype(class, ..) => {
                class.read().lookup_instance_traits(name, known_traits)
            }
            ScriptObjectClass::NoClass => Ok(()),
        }
    }

    pub fn has_trait(&self, name: &QName<'gc>) -> Result<bool, Error> {
        match &self.class {
            //Class constructors have local traits only.
            ScriptObjectClass::ClassConstructor(..) => self.provides_trait(name),

            //Prototypes do not have traits available locally, but we walk
            //through them to find traits (see `provides_trait`)
            ScriptObjectClass::InstancePrototype(..) => Ok(false),

            //Instances walk the prototype chain to build a list of known
            //traits provided by the classes attached to those prototypes.
            ScriptObjectClass::NoClass => {
                let mut proto = self.proto();

                while let Some(p) = proto {
                    if p.provides_trait(name)? {
                        return Ok(true);
                    }

                    proto = p.proto();
                }

                Ok(false)
            }
        }
    }

    pub fn provides_trait(&self, name: &QName<'gc>) -> Result<bool, Error> {
        match &self.class {
            ScriptObjectClass::ClassConstructor(class, ..) => {
                Ok(class.read().has_class_trait(name))
            }
            ScriptObjectClass::InstancePrototype(class, ..) => {
                Ok(class.read().has_instance_trait(name))
            }
            ScriptObjectClass::NoClass => Ok(false),
        }
    }

    pub fn get_scope(&self) -> Option<GcCell<'gc, Scope<'gc>>> {
        match &self.class {
            ScriptObjectClass::ClassConstructor(_class, scope) => *scope,
            ScriptObjectClass::InstancePrototype(_class, scope) => *scope,
            ScriptObjectClass::NoClass => self.proto().and_then(|proto| proto.get_scope()),
        }
    }

    pub fn resolve_any(&self, local_name: AvmString<'gc>) -> Result<Option<Namespace<'gc>>, Error> {
        for (key, _value) in self.values.iter() {
            if key.local_name() == local_name {
                return Ok(Some(key.namespace().clone()));
            }
        }

        let trait_ns = match self.class {
            ScriptObjectClass::ClassConstructor(..) => self.resolve_any_trait(local_name)?,
            ScriptObjectClass::NoClass => self.resolve_any_trait(local_name)?,
            _ => None,
        };

        if trait_ns.is_none() {
            if let Some(proto) = self.proto() {
                proto.resolve_any(local_name)
            } else {
                Ok(None)
            }
        } else {
            Ok(trait_ns)
        }
    }

    pub fn resolve_any_trait(
        &self,
        local_name: AvmString<'gc>,
    ) -> Result<Option<Namespace<'gc>>, Error> {
        if let Some(proto) = self.proto {
            let proto_trait_name = proto.resolve_any_trait(local_name)?;
            if let Some(ns) = proto_trait_name {
                return Ok(Some(ns));
            }
        }

        match &self.class {
            ScriptObjectClass::ClassConstructor(class, ..) => {
                Ok(class.read().resolve_any_class_trait(local_name))
            }
            ScriptObjectClass::InstancePrototype(class, ..) => {
                Ok(class.read().resolve_any_instance_trait(local_name))
            }
            ScriptObjectClass::NoClass => Ok(None),
        }
    }

    pub fn has_own_property(&self, name: &QName<'gc>) -> Result<bool, Error> {
        Ok(self.values.get(name).is_some() || self.has_trait(name)?)
    }

    pub fn has_instantiated_property(&self, name: &QName<'gc>) -> bool {
        self.values.get(name).is_some()
    }

    pub fn has_own_virtual_getter(&self, name: &QName<'gc>) -> bool {
        matches!(
            self.values.get(name),
            Some(Property::Virtual { get: Some(_), .. })
        )
    }

    pub fn has_own_virtual_setter(&self, name: &QName<'gc>) -> bool {
        matches!(
            self.values.get(name),
            Some(Property::Virtual { set: Some(_), .. })
        )
    }

    pub fn proto(&self) -> Option<Object<'gc>> {
        self.proto
    }

    pub fn set_proto(&mut self, proto: Object<'gc>) {
        self.proto = Some(proto)
    }

    pub fn get_enumerant_name(&self, index: u32) -> Option<QName<'gc>> {
        // NOTE: AVM2 object enumeration is one of the weakest parts of an
        // otherwise well-designed VM. Notably, because of the way they
        // implemented `hasnext` and `hasnext2`, all enumerants start from ONE.
        // Hence why we have to `checked_sub` here in case some miscompiled
        // code doesn't check for the zero index, which is actually a failure
        // sentinel.
        let true_index = (index as usize).checked_sub(1)?;

        self.enumerants.get(true_index).cloned()
    }

    pub fn property_is_enumerable(&self, name: &QName<'gc>) -> bool {
        self.enumerants.contains(name)
    }

    pub fn set_local_property_is_enumerable(
        &mut self,
        name: &QName<'gc>,
        is_enumerable: bool,
    ) -> Result<(), Error> {
        if is_enumerable && self.values.contains_key(name) && !self.enumerants.contains(name) {
            // Traits are never enumerable
            if self.has_trait(name)? {
                return Ok(());
            }

            self.enumerants.push(name.clone());
        } else if !is_enumerable && self.enumerants.contains(name) {
            let mut index = None;
            for (i, other_name) in self.enumerants.iter().enumerate() {
                if other_name == name {
                    index = Some(i);
                }
            }

            if let Some(index) = index {
                self.enumerants.remove(index);
            }
        }

        Ok(())
    }

    pub fn class(&self) -> &ScriptObjectClass<'gc> {
        &self.class
    }

    /// Install a method into the object.
    pub fn install_method(&mut self, name: QName<'gc>, disp_id: u32, function: Object<'gc>) {
        if disp_id > 0 {
            if self.methods.len() <= disp_id as usize {
                self.methods
                    .resize_with(disp_id as usize + 1, Default::default);
            }

            *self.methods.get_mut(disp_id as usize).unwrap() = Some(function);
        }

        self.values.insert(name, Property::new_method(function));
    }

    /// Install a getter into the object.
    ///
    /// This is a little more complicated than methods, since virtual property
    /// slots can be installed in two parts. Thus, we need to support
    /// installing them in either order.
    pub fn install_getter(
        &mut self,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error> {
        function
            .as_executable()
            .ok_or_else(|| Error::from("Attempted to install getter without a valid method"))?;

        if disp_id > 0 {
            if self.methods.len() <= disp_id as usize {
                self.methods
                    .resize_with(disp_id as usize + 1, Default::default);
            }

            *self.methods.get_mut(disp_id as usize).unwrap() = Some(function);
        }

        if !self.values.contains_key(&name) {
            self.values.insert(name.clone(), Property::new_virtual());
        }

        self.values
            .get_mut(&name)
            .unwrap()
            .install_virtual_getter(function)
    }

    /// Install a setter into the object.
    ///
    /// This is a little more complicated than methods, since virtual property
    /// slots can be installed in two parts. Thus, we need to support
    /// installing them in either order.
    pub fn install_setter(
        &mut self,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error> {
        function
            .as_executable()
            .ok_or_else(|| Error::from("Attempted to install setter without a valid method"))?;

        if disp_id > 0 {
            if self.methods.len() <= disp_id as usize {
                self.methods
                    .resize_with(disp_id as usize + 1, Default::default);
            }

            *self.methods.get_mut(disp_id as usize).unwrap() = Some(function);
        }

        if !self.values.contains_key(&name) {
            self.values.insert(name.clone(), Property::new_virtual());
        }

        self.values
            .get_mut(&name)
            .unwrap()
            .install_virtual_setter(function)
    }

    pub fn install_dynamic_property(
        &mut self,
        name: QName<'gc>,
        value: Value<'gc>,
    ) -> Result<(), Error> {
        self.values
            .insert(name, Property::new_dynamic_property(value));

        Ok(())
    }

    /// Install a slot onto the object.
    ///
    /// Slot number zero indicates a slot ID that is unknown and should be
    /// allocated by the VM - as far as I know, there is no way to discover
    /// slot IDs, so we don't allocate a slot for them at all.
    pub fn install_slot(&mut self, name: QName<'gc>, id: u32, value: Value<'gc>) {
        if id == 0 {
            self.values.insert(name, Property::new_stored(value));
        } else {
            self.values.insert(name, Property::new_slot(id));
            if self.slots.len() < id as usize + 1 {
                self.slots.resize_with(id as usize + 1, Default::default);
            }

            if let Some(slot) = self.slots.get_mut(id as usize) {
                *slot = Slot::new(value);
            }
        }
    }

    /// Install a const onto the object.
    ///
    /// Slot number zero indicates a slot ID that is unknown and should be
    /// allocated by the VM - as far as I know, there is no way to discover
    /// slot IDs, so we don't allocate a slot for them at all.
    pub fn install_const(&mut self, name: QName<'gc>, id: u32, value: Value<'gc>) {
        if id == 0 {
            self.values.insert(name, Property::new_const(value));
        } else {
            self.values.insert(name, Property::new_slot(id));
            if self.slots.len() < id as usize + 1 {
                self.slots.resize_with(id as usize + 1, Default::default);
            }

            if let Some(slot) = self.slots.get_mut(id as usize) {
                *slot = Slot::new_const(value);
            }
        }
    }

    /// Enumerate all interfaces implemented by this object.
    pub fn interfaces(&self) -> Vec<Object<'gc>> {
        self.interfaces.clone()
    }

    /// Set the interface list for this object.
    pub fn set_interfaces(&mut self, iface_list: Vec<Object<'gc>>) {
        self.interfaces = iface_list;
    }

    /// Get the class for this object, if it has one.
    pub fn as_class(&self) -> Option<GcCell<'gc, Class<'gc>>> {
        match self.class {
            ScriptObjectClass::ClassConstructor(class, _) => Some(class),
            ScriptObjectClass::InstancePrototype(class, _) => Some(class),
            ScriptObjectClass::NoClass => None,
        }
    }
}
