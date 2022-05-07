mod dispatch;
mod func_info;

use super::{host, Host, RawVal};
use func_info::HOST_FUNCTIONS;
use wasmi::{
    Externals, FuncInstance, ImportResolver, Module, ModuleInstance, ModuleRef, RuntimeArgs,
    RuntimeValue, ValueType,
};

impl wasmi::HostError for host::Error {}

impl Externals for Host {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
        if index > HOST_FUNCTIONS.len() {
            return Err(wasmi::TrapKind::TableAccessOutOfBounds.into());
        }
        let hf = &HOST_FUNCTIONS[index];
        if hf.arity != args.len() {
            return Err(wasmi::TrapKind::UnexpectedSignature.into());
        }

        let rv = (hf.dispatch)(self, args)?;
        Ok(Some(rv))
    }
}

impl ImportResolver for Host {
    fn resolve_func(
        &self,
        module_name: &str,
        field_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        for (i, hf) in HOST_FUNCTIONS.iter().enumerate() {
            if module_name == hf.mod_name && field_name == hf.field_name {
                if signature.params().len() != hf.arity
                    || !signature.params().iter().all(|p| *p == ValueType::I64)
                    || signature.return_type() != Some(ValueType::I64)
                {
                    return Err(wasmi::Error::Function(format!(
                        "Bad imported function signature on {}.{}",
                        module_name, field_name
                    )));
                }
                return Ok(FuncInstance::alloc_host(signature.clone(), i));
            }
        }
        Err(wasmi::Error::Function(format!(
            "No such function: {}.{}",
            module_name, field_name
        )))
    }

    fn resolve_global(
        &self,
        module_name: &str,
        field_name: &str,
        _descriptor: &wasmi::GlobalDescriptor,
    ) -> Result<wasmi::GlobalRef, wasmi::Error> {
        Err(wasmi::Error::Global(format!(
            "No such global: {}.{}",
            module_name, field_name
        )))
    }

    fn resolve_memory(
        &self,
        module_name: &str,
        field_name: &str,
        _descriptor: &wasmi::MemoryDescriptor,
    ) -> Result<wasmi::MemoryRef, wasmi::Error> {
        Err(wasmi::Error::Memory(format!(
            "No such memory: {}.{}",
            module_name, field_name
        )))
    }

    fn resolve_table(
        &self,
        module_name: &str,
        field_name: &str,
        _descriptor: &wasmi::TableDescriptor,
    ) -> Result<wasmi::TableRef, wasmi::Error> {
        Err(wasmi::Error::Table(format!(
            "No such table: {}.{}",
            module_name, field_name
        )))
    }
}

// A VM is held in a Host and contains a single WASM module instantiation.
//
// It denies modules with either floating point or start functions.
//
// In order to call construct or it, one must provide a reference to the Host,
// which will be used for looking up function references and invoking them.
// The code to do the lookups and invocation is above.
//
// Any lookups on any tables other than import functions will fail, and only
// those import functions listed above will succeed.
pub struct VM {
    instance: ModuleRef, // this is a cloneable Rc<ModuleInstance>
}

impl VM {
    pub fn new(host: &Host, module_wasm_code: &[u8]) -> Result<Self, wasmi::Error> {
        let module = Module::from_buffer(module_wasm_code)?;
        module.deny_floating_point()?;
        let not_started_instance = ModuleInstance::new(&module, host)?;
        if not_started_instance.has_start() {
            return Err(wasmi::Error::Instantiation(
                "Module has disallowed start function".into(),
            ));
        }
        let instance = not_started_instance.assert_no_start();
        Ok(Self { instance })
    }

    pub fn invoke_function(
        &self,
        host: &mut Host,
        func: &str,
        args: &[RawVal],
    ) -> Result<RawVal, wasmi::Error> {
        let wasm_args: Vec<_> = args
            .iter()
            .map(|i| RuntimeValue::I64(i.get_payload() as i64))
            .collect();
        let wasm_ret = self
            .instance
            .invoke_export(func, wasm_args.as_slice(), host)?;
        if let Some(RuntimeValue::I64(ret)) = wasm_ret {
            Ok(RawVal::from_payload(ret as u64))
        } else {
            Err(wasmi::Error::Trap(
                wasmi::TrapKind::UnexpectedSignature.into(),
            ))
        }
    }
}
