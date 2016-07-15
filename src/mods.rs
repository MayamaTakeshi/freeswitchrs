use super::*;
use super::raw as fsr;
use std::ffi::CString;
use super::raw::module_interface_name as IntName;
use std::os::raw::c_char;

pub enum Stream { } // Temp until wrap stream
pub type ApiFunc = fn(String, Option<&CoreSession>, Stream);
pub type ApiRawFunc = unsafe extern "C" fn(cmd: *const c_char,
                                           session: *mut fsr::core_session,
                                           stream: *mut fsr::stream_handle)
                                           -> fsr::status;

pub struct ModInterface(*mut fsr::loadable_module_interface);
impl ModInterface {
    gen_from_ptr!(fsr::loadable_module_interface, ModInterface, ModInterface);

    unsafe fn create_int(&self, iname: IntName) -> *mut ::std::os::raw::c_void {
        fsr::loadable_module_create_interface((*self).0, iname)
    }

    pub fn add_raw_api(&self, name: &str, desc: &str, syntax: &str, func: ApiRawFunc) {
        let name = fsr::str_to_ptr(name);
        let desc = fsr::str_to_ptr(desc);
        let syntax = fsr::str_to_ptr(syntax);
        unsafe {
            let ai = self.create_int(IntName::API_INTERFACE) as *mut fsr::api_interface;
            ptr_not_null!(ai);
            (*ai).interface_name = name;
            (*ai).desc = desc;
            (*ai).syntax = syntax;
            (*ai).function = Some(func);
        }
    }

    // Doing safe versions is a pain. Macros are ugly. Need to use libffi or similar
    // to dynamically create thunks that'll wrap the safe functions.
    // fn add_api(&mut self, name: &str, desc: &str, syntax: &str, func: ApiFunc) {
    //     self.add_raw_api(name, desc, syntax, TODO_ALLOC_TRAMPOLINE(func));
    // }
}

// Module Loading/Definition

pub struct ModDefinition {
    pub name: &'static str,
    pub load: fn(&ModInterface) -> Status,
    pub shutdown: Option<fn() -> Status>,
    pub runtime: Option<fn() -> Status>,
}

pub unsafe fn wrap_mod_load(mod_def: &ModDefinition,
                            mod_int: *mut *mut fsr::loadable_module_interface,
                            mem_pool: *mut fsr::memory_pool)
                            -> fsr::status {
    // Name should be a constant [u8], but we'd need some macro or something
    // to ensure null termination. Leaking the name here shouldn't matter.
    // CString's into_raw pointer is not free()'able fwiw
    let name = CString::new(mod_def.name).unwrap().into_raw();
    *mod_int = fsr::loadable_module_create_module_interface(mem_pool, name);
    if (*mod_int).is_null() {
        return fsr::status::MEMERR;
    }
    let mi = &ModInterface::from_ptr(*mod_int);
    (mod_def.load)(mi).to_raw()
}
// Want to end up with
// unsafe mod_skelr_load( raw shit ) { wrap_mod_load(...) }
#[macro_export]
macro_rules! freeswitch_export_mod {
    ($table:ident, $def:ident) => (
#[no_mangle]
pub unsafe extern "C" fn _mod_load(mod_int: *mut *mut fsr::loadable_module_interface,
                                        mem_pool: *mut fsr::memory_pool)
                                        -> fsr::status {
    wrap_mod_load(&$def, mod_int, mem_pool)
}
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static $table: fsr::loadable_module_function_table =
    fsr::loadable_module_function_table {
        api_version: 5,
        load: Some(_mod_load),
        shutdown: None,
        runtime: None,
        flags: fsr::module_flag_enum::NONE as u32,
    };
);}