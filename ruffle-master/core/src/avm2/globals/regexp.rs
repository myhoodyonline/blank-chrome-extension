//! `RegExp` impl

use crate::avm2::class::Class;
use crate::avm2::method::Method;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::{ArrayObject, Object, RegExpObject, TObject};
use crate::avm2::scope::Scope;
use crate::avm2::string::AvmString;
use crate::avm2::traits::Trait;
use crate::avm2::value::Value;
use crate::avm2::Error;
use crate::avm2::{activation::Activation, array::ArrayStorage};
use gc_arena::{GcCell, MutationContext};

/// Implements `RegExp`'s instance initializer.
pub fn instance_init<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;

        if let Some(mut regexp) = this.as_regexp_mut(activation.context.gc_context) {
            regexp.set_source(
                args.get(0)
                    .unwrap_or(&Value::String("".into()))
                    .coerce_to_string(activation)?,
            );

            let flags = args
                .get(1)
                .unwrap_or(&Value::String("".into()))
                .coerce_to_string(activation)?;
            for flag in flags.chars() {
                match flag {
                    's' => regexp.set_dotall(true),
                    'x' => regexp.set_extended(true),
                    'g' => regexp.set_global(true),
                    'i' => regexp.set_ignore_case(true),
                    'm' => regexp.set_multiline(true),
                    _ => {}
                };
            }
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp`'s class initializer.
pub fn class_init<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    Ok(Value::Undefined)
}

/// Implements `RegExp.dotall`
pub fn dotall<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(regexp) = this.as_regexp() {
            return Ok(regexp.dotall().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.extended`
pub fn extended<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(regexp) = this.as_regexp() {
            return Ok(regexp.extended().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.global`
pub fn global<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(regexp) = this.as_regexp() {
            return Ok(regexp.global().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.ignoreCase`
pub fn ignore_case<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(regexp) = this.as_regexp() {
            return Ok(regexp.ignore_case().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.multiline`
pub fn multiline<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(regexp) = this.as_regexp() {
            return Ok(regexp.multiline().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.lastIndex`'s getter
pub fn last_index<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(re) = this.as_regexp() {
            return Ok(re.last_index().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.lastIndex`'s setter
pub fn set_last_index<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(mut re) = this.as_regexp_mut(activation.context.gc_context) {
            let i = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_u32(activation)?;
            re.set_last_index(i as usize);
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.source`
pub fn source<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(re) = this.as_regexp() {
            return Ok(re.source().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.exec`
pub fn exec<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(mut re) = this.as_regexp_mut(activation.context.gc_context) {
            let text = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_string(activation)?;

            let (storage, index) = match re.exec(&text) {
                Some(matched) => {
                    let substrings = matched
                        .groups()
                        .map(|range| text[range.unwrap()].to_string());

                    let mut storage = ArrayStorage::new(0);
                    for substring in substrings {
                        storage
                            .push(AvmString::new(activation.context.gc_context, substring).into());
                    }

                    (storage, matched.start())
                }
                None => return Ok(Value::Null),
            };

            let object = ArrayObject::from_array(
                storage,
                activation
                    .context
                    .avm2
                    .system_prototypes
                    .as_ref()
                    .map(|sp| sp.array)
                    .unwrap(),
                activation.context.gc_context,
            );

            object.set_property_local(
                object,
                &QName::new(Namespace::public(), "index"),
                Value::Number(index as f64),
                activation,
            )?;

            object.set_property_local(
                object,
                &QName::new(Namespace::public(), "input"),
                text.into(),
                activation,
            )?;

            return Ok(object.into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `RegExp.test`
pub fn test<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Some(mut re) = this.as_regexp_mut(activation.context.gc_context) {
            let text = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_string(activation)?;
            return Ok(re.test(&text).into());
        }
    }

    Ok(Value::Undefined)
}

/// Construct `RegExp`'s class.
pub fn create_class<'gc>(mc: MutationContext<'gc, '_>) -> GcCell<'gc, Class<'gc>> {
    let class = Class::new(
        QName::new(Namespace::public(), "RegExp"),
        Some(QName::new(Namespace::public(), "Object").into()),
        Method::from_builtin(instance_init),
        Method::from_builtin(class_init),
        mc,
    );

    let mut write = class.write(mc);

    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "dotall"),
        Method::from_builtin(dotall),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "extended"),
        Method::from_builtin(extended),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "global"),
        Method::from_builtin(global),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "ignoreCase"),
        Method::from_builtin(ignore_case),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "multiline"),
        Method::from_builtin(multiline),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "lastIndex"),
        Method::from_builtin(last_index),
    ));
    write.define_instance_trait(Trait::from_setter(
        QName::new(Namespace::public(), "lastIndex"),
        Method::from_builtin(set_last_index),
    ));
    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "source"),
        Method::from_builtin(source),
    ));

    write.define_instance_trait(Trait::from_method(
        QName::new(Namespace::as3_namespace(), "exec"),
        Method::from_builtin(exec),
    ));
    write.define_instance_trait(Trait::from_method(
        QName::new(Namespace::as3_namespace(), "test"),
        Method::from_builtin(test),
    ));

    class
}

pub fn regexp_deriver<'gc>(
    base_proto: Object<'gc>,
    activation: &mut Activation<'_, 'gc, '_>,
    class: GcCell<'gc, Class<'gc>>,
    scope: Option<GcCell<'gc, Scope<'gc>>>,
) -> Result<Object<'gc>, Error> {
    Ok(RegExpObject::derive(
        base_proto,
        activation.context.gc_context,
        class,
        scope,
    ))
}
