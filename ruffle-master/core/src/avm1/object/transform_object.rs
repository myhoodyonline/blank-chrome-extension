use crate::avm1::error::Error;
use crate::avm1::{Object, ScriptObject, TDisplayObject, TObject, Value};
use crate::display_object::MovieClip;
use crate::impl_custom_object_without_set;
use gc_arena::{Collect, GcCell, MutationContext};

use crate::avm1::activation::Activation;
use std::fmt;

/// A flash.geom.Transform object
#[derive(Clone, Copy, Collect)]
#[collect(no_drop)]
pub struct TransformObject<'gc>(GcCell<'gc, TransformData<'gc>>);

#[derive(Clone, Collect)]
#[collect(no_drop)]
pub struct TransformData<'gc> {
    /// The underlying script object.
    base: ScriptObject<'gc>,
    clip: Option<MovieClip<'gc>>,
}

impl fmt::Debug for TransformObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let this = self.0.read();
        f.debug_struct("Transform")
            .field("clip", &this.clip)
            .finish()
    }
}

impl<'gc> TransformObject<'gc> {
    pub fn empty(gc_context: MutationContext<'gc, '_>, proto: Option<Object<'gc>>) -> Self {
        TransformObject(GcCell::allocate(
            gc_context,
            TransformData {
                base: ScriptObject::object(gc_context, proto),
                clip: None,
            },
        ))
    }

    pub fn clip(self) -> Option<MovieClip<'gc>> {
        self.0.read().clip
    }

    pub fn set_clip(self, gc_context: MutationContext<'gc, '_>, clip: MovieClip<'gc>) {
        self.0.write(gc_context).clip = Some(clip)
    }
}

impl<'gc> TObject<'gc> for TransformObject<'gc> {
    impl_custom_object_without_set!(base);

    fn as_transform_object(&self) -> Option<TransformObject<'gc>> {
        Some(*self)
    }

    fn construct(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        args: &[Value<'gc>],
    ) -> Result<Value<'gc>, Error<'gc>> {
        let prototype = self
            .get("prototype", activation)?
            .coerce_to_object(activation);

        let clip = args
            .get(0)
            .unwrap_or(&Value::Undefined)
            .coerce_to_object(activation)
            .as_display_object()
            .and_then(|o| o.as_movie_clip());

        let this = if clip.is_some() {
            let this = prototype.create_bare_object(activation, prototype)?;
            self.construct_on_existing(activation, this, args)?;
            this
        } else {
            // TODO: This should return an unboxed undefined.
            Value::Undefined.coerce_to_object(activation)
        };
        Ok(this.into())
    }

    #[allow(clippy::new_ret_no_self)]
    fn create_bare_object(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        this: Object<'gc>,
    ) -> Result<Object<'gc>, Error<'gc>> {
        Ok(TransformObject::empty(activation.context.gc_context, Some(this)).into())
    }

    fn set(
        &self,
        name: &str,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error<'gc>> {
        let base = self.0.read().base;
        base.internal_set(
            name,
            value,
            activation,
            (*self).into(),
            Some(activation.context.avm1.prototypes.color_transform),
        )
    }
}
