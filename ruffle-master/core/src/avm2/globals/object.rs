//! Object builtin and prototype

use crate::avm2::activation::Activation;
use crate::avm2::class::Class;
use crate::avm2::method::Method;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::{FunctionObject, Object, ScriptObject, TObject};
use crate::avm2::scope::Scope;
use crate::avm2::value::Value;
use crate::avm2::Error;
use gc_arena::{GcCell, MutationContext};

/// Implements `Object`'s instance initializer.
pub fn instance_init<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    Ok(Value::Undefined)
}

/// Implements `Object`'s class initializer
pub fn class_init<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    Ok(Value::Undefined)
}

/// Implements `Object.prototype.toString`
fn to_string<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    this.map(|t| t.to_string(activation.context.gc_context))
        .unwrap_or(Ok(Value::Undefined))
}

/// Implements `Object.prototype.toLocaleString`
fn to_locale_string<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    this.map(|t| t.to_locale_string(activation.context.gc_context))
        .unwrap_or(Ok(Value::Undefined))
}

/// Implements `Object.prototype.valueOf`
fn value_of<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    this.map(|t| t.value_of(activation.context.gc_context))
        .unwrap_or(Ok(Value::Undefined))
}

/// `Object.prototype.hasOwnProperty`
pub fn has_own_property<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    let this: Result<Object<'gc>, Error> = this.ok_or_else(|| "No valid this parameter".into());
    let this = this?;
    let name: Result<&Value<'gc>, Error> = args.get(0).ok_or_else(|| "No name specified".into());
    let name = name?.coerce_to_string(activation)?;

    if let Some(ns) = this.resolve_any(name)? {
        if !ns.is_private() {
            let qname = QName::new(ns, name);
            return Ok(this.has_own_property(&qname)?.into());
        }
    }

    Ok(false.into())
}

/// `Object.prototype.isPrototypeOf`
pub fn is_prototype_of<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    let search_proto: Result<Object<'gc>, Error> =
        this.ok_or_else(|| "No valid this parameter".into());
    let search_proto = search_proto?;
    let mut target_proto = args.get(0).cloned().unwrap_or(Value::Undefined);

    while let Value::Object(proto) = target_proto {
        if Object::ptr_eq(search_proto, proto) {
            return Ok(true.into());
        }

        target_proto = proto.proto().map(|o| o.into()).unwrap_or(Value::Undefined);
    }

    Ok(false.into())
}

/// `Object.prototype.propertyIsEnumerable`
pub fn property_is_enumerable<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    let this: Result<Object<'gc>, Error> = this.ok_or_else(|| "No valid this parameter".into());
    let this = this?;
    let name: Result<&Value<'gc>, Error> = args.get(0).ok_or_else(|| "No name specified".into());
    let name = name?.coerce_to_string(activation)?;

    if let Some(ns) = this.resolve_any(name)? {
        if !ns.is_private() {
            let qname = QName::new(ns, name);
            return Ok(this.property_is_enumerable(&qname).into());
        }
    }

    Ok(false.into())
}

/// `Object.prototype.setPropertyIsEnumerable`
pub fn set_property_is_enumerable<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    let this: Result<Object<'gc>, Error> = this.ok_or_else(|| "No valid this parameter".into());
    let this = this?;
    let name: Result<&Value<'gc>, Error> = args.get(0).ok_or_else(|| "No name specified".into());
    let name = name?.coerce_to_string(activation)?;

    if let Some(Value::Bool(is_enum)) = args.get(1) {
        if let Some(ns) = this.resolve_any(name)? {
            if !ns.is_private() {
                let qname = QName::new(ns, name);
                this.set_local_property_is_enumerable(
                    activation.context.gc_context,
                    &qname,
                    *is_enum,
                )?;
            }
        }
    }

    Ok(Value::Undefined)
}

/// Create object prototype.
///
/// This function creates a suitable class and object prototype attached to it,
/// but does not actually fill it with methods. That requires a valid function
/// prototype, and is thus done by `fill_proto` below.
pub fn create_proto<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    globals: Object<'gc>,
) -> (Object<'gc>, GcCell<'gc, Class<'gc>>) {
    let object_class = Class::new(
        QName::new(Namespace::public(), "Object"),
        None,
        Method::from_builtin(instance_init),
        Method::from_builtin(class_init),
        activation.context.gc_context,
    );

    let scope = Scope::push_scope(globals.get_scope(), globals, activation.context.gc_context);
    let proto =
        ScriptObject::bare_prototype(activation.context.gc_context, object_class, Some(scope));

    (proto, object_class)
}

/// Finish constructing `Object.prototype`, and also construct `Object`.
///
/// `__proto__` and other cross-linked properties of this object will *not*
/// be defined here. The caller of this function is responsible for linking
/// them in order to obtain a valid ECMAScript `Object` prototype.
///
/// Since Object and Function are so heavily intertwined, this function does
/// not allocate an object to store either proto. Instead, you must allocate
/// bare objects for both and let this function fill Object for you.
pub fn fill_proto<'gc>(
    gc_context: MutationContext<'gc, '_>,
    mut object_proto: Object<'gc>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::public(), "toString"),
        0,
        FunctionObject::from_builtin(gc_context, to_string, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::public(), "toLocaleString"),
        0,
        FunctionObject::from_builtin(gc_context, to_locale_string, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::public(), "valueOf"),
        0,
        FunctionObject::from_builtin(gc_context, value_of, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::as3_namespace(), "hasOwnProperty"),
        0,
        FunctionObject::from_builtin(gc_context, has_own_property, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::as3_namespace(), "isPrototypeOf"),
        0,
        FunctionObject::from_builtin(gc_context, is_prototype_of, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::as3_namespace(), "propertyIsEnumerable"),
        0,
        FunctionObject::from_builtin(gc_context, property_is_enumerable, fn_proto),
    );
    object_proto.install_method(
        gc_context,
        QName::new(Namespace::public(), "setPropertyIsEnumerable"),
        0,
        FunctionObject::from_builtin(gc_context, set_property_is_enumerable, fn_proto),
    );

    FunctionObject::from_builtin_constr(gc_context, instance_init, object_proto, fn_proto).unwrap()
}
