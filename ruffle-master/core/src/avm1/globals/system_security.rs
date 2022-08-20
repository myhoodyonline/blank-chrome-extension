use crate::avm1::activation::Activation;
use crate::avm1::error::Error;
use crate::avm1::function::{Executable, FunctionObject};
use crate::avm1::object::Object;
use crate::avm1::property::Attribute;
use crate::avm1::{AvmString, ScriptObject, TObject, Value};
use crate::avm_warn;
use gc_arena::MutationContext;
use std::convert::Into;

fn allow_domain<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "System.security.allowDomain() not implemented");
    Ok(Value::Undefined)
}

fn allow_insecure_domain<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(
        activation,
        "System.security.allowInsecureDomain() not implemented"
    );
    Ok(Value::Undefined)
}

fn load_policy_file<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(
        activation,
        "System.security.allowInsecureDomain() not implemented"
    );
    Ok(Value::Undefined)
}

fn escape_domain<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "System.security.escapeDomain() not implemented");
    Ok(Value::Undefined)
}

fn get_sandbox_type<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(AvmString::new(
        activation.context.gc_context,
        activation.context.system.sandbox_type.to_string(),
    )
    .into())
}

fn get_choose_local_swf_path<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(
        activation,
        "System.security.chooseLocalSwfPath() not implemented"
    );
    Ok(Value::Undefined)
}

fn policy_file_resolver<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(
        activation,
        "System.security.chooseLocalSwfPath() not implemented"
    );
    Ok(Value::Undefined)
}

pub fn create<'gc>(
    gc_context: MutationContext<'gc, '_>,
    proto: Option<Object<'gc>>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    let mut security = ScriptObject::object(gc_context, proto);

    security.force_set_function(
        "PolicyFileResolver",
        policy_file_resolver,
        gc_context,
        Attribute::empty(),
        Some(fn_proto),
    );

    security.force_set_function(
        "allowDomain",
        allow_domain,
        gc_context,
        Attribute::empty(),
        Some(fn_proto),
    );

    security.force_set_function(
        "allowInsecureDomain",
        allow_insecure_domain,
        gc_context,
        Attribute::empty(),
        Some(fn_proto),
    );

    security.force_set_function(
        "loadPolicyFile",
        load_policy_file,
        gc_context,
        Attribute::empty(),
        Some(fn_proto),
    );

    security.force_set_function(
        "escapeDomain",
        escape_domain,
        gc_context,
        Attribute::empty(),
        Some(fn_proto),
    );

    security.add_property(
        gc_context,
        "sandboxType",
        FunctionObject::function(
            gc_context,
            Executable::Native(get_sandbox_type),
            Some(fn_proto),
            fn_proto,
        ),
        None,
        Attribute::empty(),
    );

    security.add_property(
        gc_context,
        "chooseLocalSwfPath",
        FunctionObject::function(
            gc_context,
            Executable::Native(get_choose_local_swf_path),
            Some(fn_proto),
            fn_proto,
        ),
        None,
        Attribute::empty(),
    );

    security.into()
}
