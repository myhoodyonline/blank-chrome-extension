//! AVM2 objects.

use crate::avm2::activation::Activation;
use crate::avm2::array::ArrayStorage;
use crate::avm2::bytearray::ByteArrayStorage;
use crate::avm2::class::Class;
use crate::avm2::domain::Domain;
use crate::avm2::events::{DispatchList, Event};
use crate::avm2::function::Executable;
use crate::avm2::names::{Multiname, Namespace, QName};
use crate::avm2::regexp::RegExp;
use crate::avm2::scope::Scope;
use crate::avm2::string::AvmString;
use crate::avm2::traits::{Trait, TraitKind};
use crate::avm2::value::{Hint, Value};
use crate::avm2::Error;
use crate::display_object::DisplayObject;
use gc_arena::{Collect, GcCell, MutationContext};
use ruffle_macros::enum_trait_object;
use std::cell::{Ref, RefMut};
use std::fmt::Debug;

mod array_object;
mod bytearray_object;
mod custom_object;
mod dispatch_object;
mod domain_object;
mod event_object;
mod function_object;
mod namespace_object;
mod primitive_object;
mod regexp_object;
mod script_object;
mod stage_object;
mod xml_object;

pub use crate::avm2::object::array_object::ArrayObject;
pub use crate::avm2::object::bytearray_object::ByteArrayObject;
pub use crate::avm2::object::dispatch_object::DispatchObject;
pub use crate::avm2::object::domain_object::DomainObject;
pub use crate::avm2::object::event_object::EventObject;
pub use crate::avm2::object::function_object::{implicit_deriver, FunctionObject};
pub use crate::avm2::object::namespace_object::NamespaceObject;
pub use crate::avm2::object::primitive_object::PrimitiveObject;
pub use crate::avm2::object::regexp_object::RegExpObject;
pub use crate::avm2::object::script_object::ScriptObject;
pub use crate::avm2::object::stage_object::StageObject;
pub use crate::avm2::object::xml_object::XmlObject;

/// Represents an object that can be directly interacted with by the AVM2
/// runtime.
#[enum_trait_object(
    #[derive(Clone, Collect, Debug, Copy)]
    #[collect(no_drop)]
    pub enum Object<'gc> {
        ScriptObject(ScriptObject<'gc>),
        FunctionObject(FunctionObject<'gc>),
        PrimitiveObject(PrimitiveObject<'gc>),
        NamespaceObject(NamespaceObject<'gc>),
        ArrayObject(ArrayObject<'gc>),
        StageObject(StageObject<'gc>),
        DomainObject(DomainObject<'gc>),
        EventObject(EventObject<'gc>),
        DispatchObject(DispatchObject<'gc>),
        XmlObject(XmlObject<'gc>),
        RegExpObject(RegExpObject<'gc>),
        ByteArrayObject(ByteArrayObject<'gc>)
    }
)]
pub trait TObject<'gc>: 'gc + Collect + Debug + Into<Object<'gc>> + Clone + Copy {
    /// Retrieve a property by its QName, without taking prototype lookups
    /// into account.
    fn get_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<Value<'gc>, Error>;

    /// Retrieve a property by its QName.
    fn get_property(
        &mut self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<Value<'gc>, Error> {
        if !self.has_instantiated_property(name) {
            for abc_trait in self.get_trait(name)? {
                self.install_trait(activation, abc_trait, receiver)?;
            }
        }

        let has_no_getter = self.has_own_virtual_setter(name) && !self.has_own_virtual_getter(name);

        if self.has_own_property(name)? && !has_no_getter {
            return self.get_property_local(receiver, name, activation);
        }

        if let Some(mut proto) = self.proto() {
            return proto.get_property(receiver, name, activation);
        }

        Ok(Value::Undefined)
    }

    /// Retrieve the base prototype that a particular QName trait is defined in.
    ///
    /// This function returns `None` for non-trait properties, such as actually
    /// defined prototype methods for ES3-style classes.
    fn get_base_proto(self, name: &QName<'gc>) -> Result<Option<Object<'gc>>, Error> {
        if self.provides_trait(name)? {
            return Ok(Some(self.into()));
        }

        if let Some(proto) = self.proto() {
            return proto.get_base_proto(name);
        }

        Ok(None)
    }

    /// Set a property on this specific object.
    fn set_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error>;

    /// Set a property by its QName.
    fn set_property(
        &mut self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        if !self.has_instantiated_property(name) {
            for abc_trait in self.get_trait(name)? {
                self.install_trait(activation, abc_trait, receiver)?;
            }
        }

        if self.has_own_virtual_setter(name) {
            return self.set_property_local(receiver, name, value, activation);
        }

        let mut proto = self.proto();
        while let Some(mut my_proto) = proto {
            //NOTE: This only works because we validate ahead-of-time that
            //we're calling a virtual setter. If you call `set_property` on
            //a non-virtual you will actually alter the prototype.
            if my_proto.has_own_virtual_setter(name) {
                return my_proto.set_property(receiver, name, value, activation);
            }

            proto = my_proto.proto();
        }

        receiver.set_property_local(receiver, name, value, activation)
    }

    /// Init a property on this specific object.
    fn init_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error>;

    /// Init a property by its QName.
    fn init_property(
        &mut self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        if !self.has_instantiated_property(name) {
            for abc_trait in self.get_trait(name)? {
                self.install_trait(activation, abc_trait, receiver)?;
            }
        }

        if self.has_own_virtual_setter(name) {
            return self.init_property_local(receiver, name, value, activation);
        }

        let mut proto = self.proto();
        while let Some(mut my_proto) = proto {
            //NOTE: This only works because we validate ahead-of-time that
            //we're calling a virtual setter. If you call `set_property` on
            //a non-virtual you will actually alter the prototype.
            if my_proto.has_own_virtual_setter(name) {
                return my_proto.init_property(receiver, name, value, activation);
            }

            proto = my_proto.proto();
        }

        receiver.init_property_local(receiver, name, value, activation)
    }

    /// Retrieve a slot by its index.
    fn get_slot(self, id: u32) -> Result<Value<'gc>, Error>;

    /// Set a slot by its index.
    fn set_slot(
        self,
        id: u32,
        value: Value<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error>;

    /// Initialize a slot by its index.
    fn init_slot(
        self,
        id: u32,
        value: Value<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error>;

    /// Retrieve a method by its index.
    fn get_method(self, id: u32) -> Option<Object<'gc>>;

    /// Retrieves a trait entry by name.
    ///
    /// This function returns `None` if no such trait exists, or the object
    /// does not have traits. It returns `Err` if *any* trait in the object is
    /// malformed in some way.
    fn get_trait(self, name: &QName<'gc>) -> Result<Vec<Trait<'gc>>, Error>;

    /// Populate a list of traits that this object provides.
    ///
    /// This function yields traits for class constructors and prototypes, but
    /// not instances. For resolving traits for normal `TObject` methods, use
    /// `get_trait` and `has_trait` as it will tell you if the current object
    /// has a given trait.
    fn get_provided_trait(
        &self,
        name: &QName<'gc>,
        known_traits: &mut Vec<Trait<'gc>>,
    ) -> Result<(), Error>;

    /// Retrieves the scope chain of the object at time of its creation.
    ///
    /// The scope chain is used to determine the starting scope stack when an
    /// object is called, as well as any class methods on the object.
    /// Non-method functions and prototype functions (ES3 methods) do not use
    /// this scope chain.
    fn get_scope(self) -> Option<GcCell<'gc, Scope<'gc>>>;

    /// Resolve a multiname into a single QName, if any of the namespaces
    /// match.
    fn resolve_multiname(self, multiname: &Multiname<'gc>) -> Result<Option<QName<'gc>>, Error> {
        for ns in multiname.namespace_set() {
            if ns.is_any() {
                if let Some(name) = multiname.local_name() {
                    let ns = self.resolve_any(name)?;
                    return Ok(ns.map(|ns| QName::new(ns, name)));
                } else {
                    return Ok(None);
                }
            } else if let Some(name) = multiname.local_name() {
                let qname = QName::new(ns.clone(), name);
                if self.has_property(&qname)? {
                    return Ok(Some(qname));
                }
            } else {
                return Ok(None);
            }
        }

        if let Some(proto) = self.proto() {
            return proto.resolve_multiname(multiname);
        }

        Ok(None)
    }

    /// Given a local name, find the namespace it resides in, if any.
    ///
    /// The `Namespace` must not be `Namespace::Any`, as this function exists
    /// specifically resolve names in that namespace.
    ///
    /// Trait names will be resolve on class constructors and object instances,
    /// but not prototypes. If you want to search a prototype's provided traits
    /// you must walk the prototype chain using `resolve_any_trait`.
    fn resolve_any(self, local_name: AvmString<'gc>) -> Result<Option<Namespace<'gc>>, Error>;

    /// Given a local name of a trait, find the namespace it resides in, if any.
    ///
    /// This function only works for names which are trait properties, not
    /// dynamic or prototype properties. Furthermore, instance prototypes *will*
    /// resolve trait names here, contrary to their behavior in `resolve_any.`
    fn resolve_any_trait(self, local_name: AvmString<'gc>)
        -> Result<Option<Namespace<'gc>>, Error>;

    /// Indicates whether or not a property exists on an object.
    fn has_property(self, name: &QName<'gc>) -> Result<bool, Error> {
        if self.has_own_property(name)? {
            Ok(true)
        } else if let Some(proto) = self.proto() {
            Ok(proto.has_own_property(name)?)
        } else {
            Ok(false)
        }
    }

    /// Indicates whether or not a property or trait exists on an object and is
    /// not part of the prototype chain.
    fn has_own_property(self, name: &QName<'gc>) -> Result<bool, Error>;

    /// Returns true if an object has one or more traits of a given name.
    fn has_trait(self, name: &QName<'gc>) -> Result<bool, Error>;

    /// Returns true if an object is part of a class that defines a trait of a
    /// given name on itself (as opposed to merely inheriting a superclass
    /// trait.)
    fn provides_trait(self, name: &QName<'gc>) -> Result<bool, Error>;

    /// Indicates whether or not a property or *instantiated* trait exists on
    /// an object and is not part of the prototype chain.
    ///
    /// Unlike `has_own_property`, this will not yield `true` for traits this
    /// object can have but has not yet instantiated.
    fn has_instantiated_property(self, name: &QName<'gc>) -> bool;

    /// Check if a particular object contains a virtual getter by the given
    /// name.
    fn has_own_virtual_getter(self, name: &QName<'gc>) -> bool;

    /// Check if a particular object contains a virtual setter by the given
    /// name.
    fn has_own_virtual_setter(self, name: &QName<'gc>) -> bool;

    /// Indicates whether or not a property is overwritable.
    fn is_property_overwritable(
        self,
        gc_context: MutationContext<'gc, '_>,
        _name: &QName<'gc>,
    ) -> bool;

    /// Delete a named property from the object.
    ///
    /// Returns false if the property cannot be deleted.
    fn delete_property(&self, gc_context: MutationContext<'gc, '_>, name: &QName<'gc>) -> bool;

    /// Retrieve the `__proto__` of a given object.
    ///
    /// The proto is another object used to resolve methods across a class of
    /// multiple objects. It should also be accessible as `__proto__` from
    /// `get`.
    fn proto(&self) -> Option<Object<'gc>>;

    /// Change the `__proto__` on this object.
    ///
    /// This method primarily exists so that the global scope that player
    /// globals loads into can be created before its superclasses are. It
    /// should be used sparingly, if at all.
    fn set_proto(self, mc: MutationContext<'gc, '_>, proto: Object<'gc>);

    /// Retrieve a given enumerable name by index.
    ///
    /// Enumerants are listed by index, starting from zero. A value of `None`
    /// indicates that no enumerant with that index, or any greater index,
    /// exists. (In other words, it means stop.)
    ///
    /// Objects are responsible for maintaining a consistently ordered and
    /// indexed list of enumerable names which can be queried by this
    /// mechanism.
    fn get_enumerant_name(&self, index: u32) -> Option<QName<'gc>>;

    /// Determine if a property is currently enumerable.
    ///
    /// Properties that do not exist are also not enumerable.
    fn property_is_enumerable(&self, name: &QName<'gc>) -> bool;

    /// Mark a dynamic property on this object as enumerable.
    fn set_local_property_is_enumerable(
        &self,
        mc: MutationContext<'gc, '_>,
        name: &QName<'gc>,
        is_enumerable: bool,
    ) -> Result<(), Error>;

    /// Install a method (or any other non-slot value) on an object.
    fn install_method(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    );

    /// Install a getter method on an object property.
    fn install_getter(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error>;

    /// Install a setter method on an object property.
    fn install_setter(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        disp_id: u32,
        function: Object<'gc>,
    ) -> Result<(), Error>;

    /// Install a dynamic or built-in value property on an object.
    fn install_dynamic_property(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        value: Value<'gc>,
    ) -> Result<(), Error>;

    /// Install a slot on an object property.
    fn install_slot(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        id: u32,
        value: Value<'gc>,
    );

    /// Install a const on an object property.
    fn install_const(
        &mut self,
        mc: MutationContext<'gc, '_>,
        name: QName<'gc>,
        id: u32,
        value: Value<'gc>,
    );

    /// Install a trait from the current object.
    ///
    /// This function should only be called once, as reinstalling a trait may
    /// also unset already set properties. It may either be called immediately
    /// when the object is instantiated or lazily; this behavior is ostensibly
    /// controlled by the `lazy_init` flag provided to `load_abc`, but in
    /// practice every version of Flash and Animate uses lazy trait
    /// installation.
    ///
    /// The `reciever` property allows specifying the object that methods are
    /// bound to. It should always be `self` except when doing things with
    /// `super`, which needs to create bound methods pointing to a different
    /// object.
    ///
    /// The value of the trait at the time of installation will be returned
    /// here.
    fn install_trait(
        &mut self,
        activation: &mut Activation<'_, 'gc, '_>,
        trait_entry: Trait<'gc>,
        receiver: Object<'gc>,
    ) -> Result<Value<'gc>, Error> {
        self.install_foreign_trait(activation, trait_entry, self.get_scope(), receiver)
    }

    /// Install a trait from anywyere.
    ///
    /// The value of the trait at the time of installation will be returned
    /// here.
    fn install_foreign_trait(
        &mut self,
        activation: &mut Activation<'_, 'gc, '_>,
        trait_entry: Trait<'gc>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
        receiver: Object<'gc>,
    ) -> Result<Value<'gc>, Error> {
        let fn_proto = activation.avm2().prototypes().function;
        let trait_name = trait_entry.name().clone();
        avm_debug!(
            activation.avm2(),
            "Installing trait {:?} of kind {:?}",
            trait_name,
            trait_entry.kind()
        );

        match trait_entry.kind() {
            TraitKind::Slot {
                slot_id,
                default_value,
                ..
            } => {
                let value = default_value.clone().unwrap_or(Value::Undefined);
                self.install_slot(
                    activation.context.gc_context,
                    trait_name,
                    *slot_id,
                    value.clone(),
                );

                Ok(value)
            }
            TraitKind::Method {
                disp_id, method, ..
            } => {
                let function = FunctionObject::from_method(
                    activation.context.gc_context,
                    method.clone(),
                    scope,
                    fn_proto,
                    Some(receiver),
                );
                self.install_method(
                    activation.context.gc_context,
                    trait_name,
                    *disp_id,
                    function,
                );

                Ok(function.into())
            }
            TraitKind::Getter {
                disp_id, method, ..
            } => {
                let function = FunctionObject::from_method(
                    activation.context.gc_context,
                    method.clone(),
                    scope,
                    fn_proto,
                    Some(receiver),
                );
                self.install_getter(
                    activation.context.gc_context,
                    trait_name,
                    *disp_id,
                    function,
                )?;

                Ok(function.into())
            }
            TraitKind::Setter {
                disp_id, method, ..
            } => {
                let function = FunctionObject::from_method(
                    activation.context.gc_context,
                    method.clone(),
                    scope,
                    fn_proto,
                    Some(receiver),
                );
                self.install_setter(
                    activation.context.gc_context,
                    trait_name,
                    *disp_id,
                    function,
                )?;

                Ok(function.into())
            }
            TraitKind::Class { slot_id, class } => {
                let class_read = class.read();
                let super_class = if let Some(sc_name) = class_read.super_class_name() {
                    let super_name = self
                        .resolve_multiname(sc_name)?
                        .unwrap_or_else(|| QName::dynamic_name("Object"));

                    let super_class = if let Some(scope) = scope {
                        scope
                            .write(activation.context.gc_context)
                            .resolve(&super_name.clone().into(), activation)?
                    } else {
                        None
                    };

                    Some(
                        super_class
                            .ok_or_else(|| {
                                format!(
                                    "Could not resolve superclass {:?}",
                                    super_name.local_name()
                                )
                            })?
                            .coerce_to_object(activation)?,
                    )
                } else {
                    None
                };

                let (class_object, _cinit) =
                    FunctionObject::from_class(activation, *class, super_class, scope)?;
                self.install_const(
                    activation.context.gc_context,
                    class_read.name().clone(),
                    *slot_id,
                    class_object.into(),
                );

                Ok(class_object.into())
            }
            TraitKind::Function {
                slot_id, function, ..
            } => {
                let mut fobject = FunctionObject::from_method(
                    activation.context.gc_context,
                    function.clone(),
                    scope,
                    fn_proto,
                    None,
                );
                let es3_proto = ScriptObject::object(
                    activation.context.gc_context,
                    activation.avm2().prototypes().object,
                );

                fobject.install_slot(
                    activation.context.gc_context,
                    QName::new(Namespace::public(), "prototype"),
                    0,
                    es3_proto.into(),
                );
                self.install_const(
                    activation.context.gc_context,
                    trait_name,
                    *slot_id,
                    fobject.into(),
                );

                Ok(fobject.into())
            }
            TraitKind::Const {
                slot_id,
                default_value,
                ..
            } => {
                let value = default_value.clone().unwrap_or(Value::Undefined);
                self.install_const(
                    activation.context.gc_context,
                    trait_name,
                    *slot_id,
                    value.clone(),
                );

                Ok(value)
            }
        }
    }

    /// Call the object.
    fn call(
        self,
        _reciever: Option<Object<'gc>>,
        _arguments: &[Value<'gc>],
        _activation: &mut Activation<'_, 'gc, '_>,
        _base_proto: Option<Object<'gc>>,
    ) -> Result<Value<'gc>, Error> {
        Err("Object is not callable".into())
    }

    /// Construct a host object of some kind and return its cell.
    ///
    /// As the first step in object construction, the `construct` method is
    /// called on the prototype to create a new object. The prototype may
    /// construct any object implementation it wants, however, it's expected
    /// to produce a like `TObject` implementor with itself as the new object's
    /// proto.
    ///
    /// After construction, the constructor function is `call`ed with the new
    /// object as `this` to initialize the object.
    ///
    /// `construct`ed objects should instantiate instance traits of the class
    /// that this prototype represents.
    ///
    /// The arguments passed to the constructor are provided here; however, all
    /// object construction should happen in `call`, not `new`. `new` exists
    /// purely so that host objects can be constructed by the VM.
    fn construct(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        args: &[Value<'gc>],
    ) -> Result<Object<'gc>, Error>;

    /// Construct a host object prototype of some kind and return it.
    ///
    /// This is called specifically to construct prototypes. The primary
    /// difference is that a new class and scope closure are defined here.
    /// Objects constructed from the new prototype should use that new class
    /// and scope closure when instantiating non-prototype traits.
    ///
    /// Unlike `construct`, `derive`d objects should *not* instantiate instance
    /// traits.
    fn derive(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Result<Object<'gc>, Error>;

    /// Determine the type of primitive coercion this object would prefer, in
    /// the case that there is no obvious reason to prefer one type over the
    /// other.
    ///
    /// All native ECMAScript objects prefer numerical coercions, except `Date`,
    /// which wants string coercions.
    fn default_hint(&self) -> Hint {
        Hint::Number
    }

    /// Implement the result of calling `Object.prototype.toString` on this
    /// object class.
    ///
    /// `toString` is a method used to request an object be coerced to a string
    /// value. The default implementation is stored here. User-specified string
    /// coercions happen by defining `toString` in a downstream class or
    /// prototype; this is then picked up by the VM runtime when doing
    /// coercions.
    fn to_string(&self, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        let class_name = self
            .as_proto_class()
            .map(|c| c.read().name().local_name())
            .unwrap_or_else(|| "Object".into());

        Ok(AvmString::new(mc, format!("[object {}]", class_name)).into())
    }

    /// Implement the result of calling `Object.prototype.toLocaleString` on this
    /// object class.
    ///
    /// `toLocaleString` is a method used to request an object be coerced to a
    /// locale-dependent string value. The default implementation appears to
    /// generate a debug-style string based on the name of the class this
    /// object is, in the format of `[object Class]` (where `Class` is the name
    /// of the class that created this object).
    fn to_locale_string(&self, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        let class_name = self
            .as_proto_class()
            .map(|c| c.read().name().local_name())
            .unwrap_or_else(|| "Object".into());

        Ok(AvmString::new(mc, format!("[object {}]", class_name)).into())
    }

    /// Implement the result of calling `Object.prototype.valueOf` on this
    /// object class.
    ///
    /// `valueOf` is a method used to request an object be coerced to a
    /// primitive value. Typically, this would be a number of some kind.
    fn value_of(&self, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error>;

    /// Enumerate all interfaces implemented by this object.
    fn interfaces(&self) -> Vec<Object<'gc>>;

    /// Set the interface list for this object.
    fn set_interfaces(&self, gc_context: MutationContext<'gc, '_>, iface_list: Vec<Object<'gc>>);

    /// Determine if this object is an instance of a given type.
    ///
    /// The given object should be the constructor for the given type we are
    /// checking against this object. Its prototype will be searched in the
    /// prototype chain of this object. If `check_interfaces` is enabled, then
    /// the interfaces listed on each prototype will also be checked.
    #[allow(unused_mut)] //it's not unused
    fn is_instance_of(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        mut constructor: Object<'gc>,
        check_interfaces: bool,
    ) -> Result<bool, Error> {
        let type_proto = constructor
            .get_property(constructor, &QName::dynamic_name("prototype"), activation)?
            .coerce_to_object(activation)?;

        self.has_prototype_in_chain(type_proto, check_interfaces)
    }

    /// Determine if this object has a given prototype in its prototype chain.
    ///
    /// The given object should be the prototype we are checking against this
    /// object. Its prototype will be searched in the
    /// prototype chain of this object. If `check_interfaces` is enabled, then
    /// the interfaces listed on each prototype will also be checked.
    fn has_prototype_in_chain(
        &self,
        type_proto: Object<'gc>,
        check_interfaces: bool,
    ) -> Result<bool, Error> {
        let mut my_proto = self.proto();

        //TODO: Is it a verification error to do `obj instanceof bare_object`?
        while let Some(proto) = my_proto {
            if Object::ptr_eq(proto, type_proto) {
                return Ok(true);
            }

            if check_interfaces {
                for interface in proto.interfaces() {
                    if Object::ptr_eq(interface, type_proto) {
                        return Ok(true);
                    }
                }
            }

            my_proto = proto.proto()
        }

        Ok(false)
    }

    /// Get a raw pointer value for this object.
    fn as_ptr(&self) -> *const ObjectPtr;

    /// Get this object's `Class`, if it has one.
    fn as_class(&self) -> Option<GcCell<'gc, Class<'gc>>>;

    /// Get this object's `Class`, or any `Class` on its prototype chain.
    ///
    /// This only yields `None` for bare objects.
    fn as_proto_class(&self) -> Option<GcCell<'gc, Class<'gc>>> {
        let mut class = self.as_class();

        while class.is_none() {
            if let Some(proto) = self.proto() {
                class = proto.as_class();
            } else {
                return None;
            }
        }

        class
    }

    /// Get this object's `Executable`, if it has one.
    fn as_executable(&self) -> Option<Executable<'gc>> {
        None
    }

    /// Unwrap this object's `Namespace`, if the object is a boxed namespace.
    fn as_namespace(&self) -> Option<Ref<Namespace<'gc>>> {
        None
    }

    /// Unwrap this object as array storage.
    fn as_array_storage(&self) -> Option<Ref<ArrayStorage<'gc>>> {
        None
    }

    /// Unwrap this object as bytearray.
    fn as_bytearray(&self) -> Option<Ref<ByteArrayStorage>> {
        None
    }

    fn as_bytearray_mut(&self, _mc: MutationContext<'gc, '_>) -> Option<RefMut<ByteArrayStorage>> {
        None
    }
    /// Unwrap this object as mutable array storage.
    fn as_array_storage_mut(
        &self,
        _mc: MutationContext<'gc, '_>,
    ) -> Option<RefMut<ArrayStorage<'gc>>> {
        None
    }

    /// Get this object's `DisplayObject`, if it has one.
    fn as_display_object(&self) -> Option<DisplayObject<'gc>> {
        None
    }

    /// Associate this object with a display object, if it can support such an
    /// association.
    ///
    /// If not, then this function does nothing.
    fn init_display_object(&self, _mc: MutationContext<'gc, '_>, _obj: DisplayObject<'gc>) {}

    /// Unwrap this object as an ApplicationDomain.
    fn as_application_domain(&self) -> Option<Domain<'gc>> {
        None
    }

    /// Unwrap this object as an event.
    fn as_event(&self) -> Option<Ref<Event<'gc>>> {
        None
    }

    /// Unwrap this object as a mutable event.
    fn as_event_mut(&self, _mc: MutationContext<'gc, '_>) -> Option<RefMut<Event<'gc>>> {
        None
    }

    /// Unwrap this object as a list of event handlers.
    fn as_dispatch(&self) -> Option<Ref<DispatchList<'gc>>> {
        None
    }

    /// Unwrap this object as a mutable list of event handlers.
    fn as_dispatch_mut(&self, _mc: MutationContext<'gc, '_>) -> Option<RefMut<DispatchList<'gc>>> {
        None
    }

    /// Unwrap this object as a mutable primitive value.
    fn as_primitive_mut(&self, _mc: MutationContext<'gc, '_>) -> Option<RefMut<Value<'gc>>> {
        None
    }

    /// Unwrap this object as a regexp.
    fn as_regexp(&self) -> Option<Ref<RegExp<'gc>>> {
        None
    }

    /// Unwrap this object as a mutable regexp.
    fn as_regexp_mut(&self, _mc: MutationContext<'gc, '_>) -> Option<RefMut<RegExp<'gc>>> {
        None
    }
}

pub enum ObjectPtr {}

impl<'gc> Object<'gc> {
    pub fn ptr_eq(a: Object<'gc>, b: Object<'gc>) -> bool {
        a.as_ptr() == b.as_ptr()
    }
}
