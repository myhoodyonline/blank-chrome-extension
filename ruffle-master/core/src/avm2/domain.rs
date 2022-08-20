//! Application Domains

use crate::avm2::activation::Activation;
use crate::avm2::names::{Multiname, QName};
use crate::avm2::object::TObject;
use crate::avm2::script::Script;
use crate::avm2::value::Value;
use crate::avm2::Error;
use gc_arena::{Collect, GcCell, MutationContext};
use std::collections::HashMap;

/// Represents a set of scripts and movies that share traits across different
/// script-global scopes.
#[derive(Copy, Clone, Debug, Collect)]
#[collect(no_drop)]
pub struct Domain<'gc>(GcCell<'gc, DomainData<'gc>>);

#[derive(Clone, Debug, Collect)]
#[collect(no_drop)]
struct DomainData<'gc> {
    /// A list of all exported definitions and the script that exported them.
    defs: HashMap<QName<'gc>, Script<'gc>>,

    /// The parent domain.
    parent: Option<Domain<'gc>>,
}

impl<'gc> Domain<'gc> {
    /// Create a new domain with no parent.
    ///
    /// This is intended exclusively for creating the player globals domain,
    /// hence the name.
    pub fn global_domain(mc: MutationContext<'gc, '_>) -> Domain<'gc> {
        Self(GcCell::allocate(
            mc,
            DomainData {
                defs: HashMap::new(),
                parent: None,
            },
        ))
    }

    /// Create a new domain with a given parent.
    pub fn movie_domain(mc: MutationContext<'gc, '_>, parent: Domain<'gc>) -> Domain<'gc> {
        Self(GcCell::allocate(
            mc,
            DomainData {
                defs: HashMap::new(),
                parent: Some(parent),
            },
        ))
    }

    /// Get the parent of this domain
    pub fn parent_domain(self) -> Option<Domain<'gc>> {
        self.0.read().parent
    }

    /// Determine if something has been defined within the current domain.
    pub fn has_definition(self, name: QName<'gc>) -> bool {
        let read = self.0.read();

        if read.defs.contains_key(&name) {
            return true;
        }

        if let Some(parent) = read.parent {
            return parent.has_definition(name);
        }

        false
    }

    /// Resolve a QName and return the script that provided it.
    ///
    /// If a name does not exist or cannot be resolved, no script or name will
    /// be returned.
    pub fn get_defining_script(
        self,
        multiname: &Multiname<'gc>,
    ) -> Result<Option<(QName<'gc>, Script<'gc>)>, Error> {
        let read = self.0.read();

        for ns in multiname.namespace_set() {
            if ns.is_any() {
                if let Some(local_name) = multiname.local_name() {
                    for (qname, script) in read.defs.iter() {
                        if qname.local_name() == local_name {
                            return Ok(Some((qname.clone(), *script)));
                        }
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(name) = multiname.local_name() {
                let qname = QName::new(ns.clone(), name);
                if read.defs.contains_key(&qname) {
                    let script = read.defs.get(&qname).cloned().unwrap();
                    return Ok(Some((qname, script)));
                }
            } else {
                return Ok(None);
            }
        }

        if let Some(parent) = read.parent {
            return parent.get_defining_script(multiname);
        }

        Ok(None)
    }

    /// Retrieve a value from this domain.
    pub fn get_defined_value(
        self,
        activation: &mut Activation<'_, 'gc, '_>,
        name: QName<'gc>,
    ) -> Result<Value<'gc>, Error> {
        let (name, mut script) = self
            .get_defining_script(&name.clone().into())?
            .ok_or_else(|| format!("MovieClip Symbol {} does not exist", name.local_name()))?;
        let mut globals = script.globals(&mut activation.context)?;

        globals.get_property(globals, &name, activation)
    }

    /// Export a definition from a script into the current application domain.
    ///
    /// This returns an error if the name is already defined in the current or
    /// any parent domains.
    pub fn export_definition(
        &mut self,
        name: QName<'gc>,
        script: Script<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<(), Error> {
        if self.has_definition(name.clone()) {
            return Err(format!(
                "VerifyError: Attempted to redefine existing name {}",
                name.local_name()
            )
            .into());
        }

        self.0.write(mc).defs.insert(name, script);

        Ok(())
    }
}
