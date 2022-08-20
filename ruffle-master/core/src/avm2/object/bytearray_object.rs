use crate::avm2::activation::Activation;
use crate::avm2::bytearray::ByteArrayStorage;
use crate::avm2::class::Class;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::script_object::{ScriptObjectClass, ScriptObjectData};
use crate::avm2::object::{Object, ObjectPtr, TObject};
use crate::avm2::scope::Scope;
use crate::avm2::string::AvmString;
use crate::avm2::traits::Trait;
use crate::avm2::value::Value;
use crate::avm2::Error;
use crate::impl_avm2_custom_object;
use gc_arena::{Collect, GcCell, MutationContext};
use std::cell::{Ref, RefMut};

#[derive(Clone, Collect, Debug, Copy)]
#[collect(no_drop)]
pub struct ByteArrayObject<'gc>(GcCell<'gc, ByteArrayObjectData<'gc>>);

#[derive(Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct ByteArrayObjectData<'gc> {
    /// Base script object
    base: ScriptObjectData<'gc>,

    storage: ByteArrayStorage,
}

impl<'gc> ByteArrayObject<'gc> {
    pub fn construct(mc: MutationContext<'gc, '_>, base_proto: Option<Object<'gc>>) -> Object<'gc> {
        let base = ScriptObjectData::base_new(base_proto, ScriptObjectClass::NoClass);

        ByteArrayObject(GcCell::allocate(
            mc,
            ByteArrayObjectData {
                base,
                storage: ByteArrayStorage::new(),
            },
        ))
        .into()
    }

    pub fn derive(
        base_proto: Object<'gc>,
        mc: MutationContext<'gc, '_>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Result<Object<'gc>, Error> {
        let base = ScriptObjectData::base_new(
            Some(base_proto),
            ScriptObjectClass::InstancePrototype(class, scope),
        );

        Ok(ByteArrayObject(GcCell::allocate(
            mc,
            ByteArrayObjectData {
                base,
                storage: ByteArrayStorage::new(),
            },
        ))
        .into())
    }
}
impl<'gc> TObject<'gc> for ByteArrayObject<'gc> {
    impl_avm2_custom_object!(base);

    fn get_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<Value<'gc>, Error> {
        let read = self.0.read();

        if name.namespace().is_public() {
            if let Ok(index) = name.local_name().parse::<usize>() {
                return Ok(if let Some(val) = read.storage.get(index) {
                    Value::Unsigned(val as u32)
                } else {
                    Value::Undefined
                });
            }
        }

        let rv = read.base.get_property_local(receiver, name, activation)?;

        drop(read);

        rv.resolve(activation)
    }

    fn set_property_local(
        self,
        receiver: Object<'gc>,
        name: &QName<'gc>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        let mut write = self.0.write(activation.context.gc_context);

        if name.namespace().is_public() {
            if let Ok(index) = name.local_name().parse::<usize>() {
                write
                    .storage
                    .set(index, value.coerce_to_u32(activation)? as u8);

                return Ok(());
            }
        }

        let rv = write
            .base
            .set_property_local(receiver, name, value, activation)?;

        drop(write);

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
        let mut write = self.0.write(activation.context.gc_context);

        if name.namespace().is_public() {
            if let Ok(index) = name.local_name().parse::<usize>() {
                write
                    .storage
                    .set(index, value.coerce_to_u32(activation)? as u8);

                return Ok(());
            }
        }

        let rv = write
            .base
            .init_property_local(receiver, name, value, activation)?;

        drop(write);

        rv.resolve(activation)?;

        Ok(())
    }

    fn is_property_overwritable(
        self,
        gc_context: MutationContext<'gc, '_>,
        name: &QName<'gc>,
    ) -> bool {
        self.0.write(gc_context).base.is_property_overwritable(name)
    }

    fn delete_property(&self, gc_context: MutationContext<'gc, '_>, name: &QName<'gc>) -> bool {
        if name.namespace().is_public() {
            if let Ok(index) = name.local_name().parse::<usize>() {
                self.0.write(gc_context).storage.delete(index);
                return true;
            }
        }

        self.0.write(gc_context).base.delete_property(name)
    }

    fn has_own_property(self, name: &QName<'gc>) -> Result<bool, Error> {
        if name.namespace().is_public() {
            if let Ok(index) = name.local_name().parse::<usize>() {
                return Ok(self.0.read().storage.get(index).is_some());
            }
        }

        self.0.read().base.has_own_property(name)
    }

    fn resolve_any(self, local_name: AvmString<'gc>) -> Result<Option<Namespace<'gc>>, Error> {
        if let Ok(index) = local_name.parse::<usize>() {
            if self.0.read().storage.get(index).is_some() {
                return Ok(Some(Namespace::public()));
            }
        }

        self.0.read().base.resolve_any(local_name)
    }

    fn resolve_any_trait(
        self,
        local_name: AvmString<'gc>,
    ) -> Result<Option<Namespace<'gc>>, Error> {
        self.0.read().base.resolve_any_trait(local_name)
    }

    fn construct(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        _args: &[Value<'gc>],
    ) -> Result<Object<'gc>, Error> {
        let this: Object<'gc> = Object::ByteArrayObject(*self);
        Ok(ByteArrayObject::construct(
            activation.context.gc_context,
            Some(this),
        ))
    }

    fn derive(
        &self,
        activation: &mut Activation<'_, 'gc, '_>,
        class: GcCell<'gc, Class<'gc>>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Result<Object<'gc>, Error> {
        let this: Object<'gc> = Object::ByteArrayObject(*self);
        let base = ScriptObjectData::base_new(
            Some(this),
            ScriptObjectClass::InstancePrototype(class, scope),
        );

        Ok(ByteArrayObject(GcCell::allocate(
            activation.context.gc_context,
            ByteArrayObjectData {
                base,
                storage: ByteArrayStorage::new(),
            },
        ))
        .into())
    }
    fn value_of(&self, _mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        Ok(Value::Object(Object::from(*self)))
    }

    fn as_bytearray(&self) -> Option<Ref<ByteArrayStorage>> {
        Some(Ref::map(self.0.read(), |d| &d.storage))
    }

    fn as_bytearray_mut(&self, mc: MutationContext<'gc, '_>) -> Option<RefMut<ByteArrayStorage>> {
        Some(RefMut::map(self.0.write(mc), |d| &mut d.storage))
    }
}
