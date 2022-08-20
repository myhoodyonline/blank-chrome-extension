//! `flash.display.Shape` builtin/prototype

use crate::avm2::activation::Activation;
use crate::avm2::class::Class;
use crate::avm2::globals::NS_RUFFLE_INTERNAL;
use crate::avm2::method::Method;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::{Object, StageObject, TObject};
use crate::avm2::traits::Trait;
use crate::avm2::value::Value;
use crate::avm2::Error;
use crate::display_object::Graphic;
use crate::tag_utils::SwfMovie;
use crate::vminterface::AvmType;
use gc_arena::{GcCell, MutationContext};
use std::sync::Arc;

/// Implements `flash.display.Shape`'s instance constructor.
pub fn instance_init<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;

        if this.as_display_object().is_none() {
            let movie = Arc::new(SwfMovie::empty(activation.context.swf.version()));
            let library = activation.context.library.library_for_movie_mut(movie);
            library.force_avm_type(AvmType::Avm2);

            let new_do = Graphic::new_with_avm2(&mut activation.context, this);

            this.init_display_object(activation.context.gc_context, new_do.into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `flash.display.Shape`'s class constructor.
pub fn class_init<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    Ok(Value::Undefined)
}

/// Implements `graphics`.
pub fn graphics<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(mut this) = this {
        if let Some(dobj) = this.as_display_object() {
            // Lazily initialize the `Graphics` object in a hidden property.
            let graphics = match this.get_property(
                this,
                &QName::new(Namespace::private(NS_RUFFLE_INTERNAL), "graphics"),
                activation,
            )? {
                Value::Undefined | Value::Null => {
                    let graphics_proto = activation.context.avm2.prototypes().graphics;
                    let graphics = Value::from(StageObject::for_display_object(
                        activation.context.gc_context,
                        dobj,
                        graphics_proto,
                    ));
                    this.set_property(
                        this,
                        &QName::new(Namespace::private(NS_RUFFLE_INTERNAL), "graphics"),
                        graphics.clone(),
                        activation,
                    )?;
                    graphics
                }
                graphics => graphics,
            };
            return Ok(graphics);
        }
    }

    Ok(Value::Undefined)
}

/// Construct `Shape`'s class.
pub fn create_class<'gc>(mc: MutationContext<'gc, '_>) -> GcCell<'gc, Class<'gc>> {
    let class = Class::new(
        QName::new(Namespace::package("flash.display"), "Shape"),
        Some(QName::new(Namespace::package("flash.display"), "DisplayObject").into()),
        Method::from_builtin(instance_init),
        Method::from_builtin(class_init),
        mc,
    );

    let mut write = class.write(mc);

    write.define_instance_trait(Trait::from_getter(
        QName::new(Namespace::public(), "graphics"),
        Method::from_builtin(graphics),
    ));

    // Slot for lazy-initialized Graphics object.
    write.define_instance_trait(Trait::from_slot(
        QName::new(Namespace::private(NS_RUFFLE_INTERNAL), "graphics"),
        QName::new(Namespace::package("flash.display"), "Graphics").into(),
        None,
    ));

    class
}
