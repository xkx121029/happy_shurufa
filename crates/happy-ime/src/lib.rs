//! DLL 入口与 COM 类工厂。

mod context;
mod global;
mod logger;
mod profile;
mod register;
mod tsf;
mod window;

use std::ffi::c_void;

use windows::Win32::Foundation::{HINSTANCE, S_FALSE, S_OK};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::core::{BOOL, GUID, IUnknown, Interface, implement};

use crate::tsf::TextService;

/// 所有 FFI 出口的崩溃防护：DLL 注入所有文本进程，一次 panic 就会崩掉宿主应用。
macro_rules! guarded {
    ($body:expr) => {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(v) => v,
            Err(_) => {
                log::error!("FFI 边界捕获到 panic，已防御性忽略");
                Default::default()
            }
        }
    };
}

#[no_mangle]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: u32, _reserved: *mut c_void) -> BOOL {
    // DllMain 本身已在 loader 锁下运行，catch_unwind 提供崩溃防护
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if call_reason != DLL_PROCESS_ATTACH {
            return BOOL::from(true);
        }
        logger::setup();
        global::setup(dll_module);
        window::setup().into()
    })) {
        Ok(v) => v,
        Err(_) => {
            log::error!("FFI 边界捕获到 panic，已防御性忽略");
            Default::default()
        }
    }
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> windows::core::HRESULT {
    guarded!(match register::register_all() {
        Ok(()) => S_OK,
        Err(e) => {
            log::error!("注册失败: {e}");
            e.into()
        }
    })
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> windows::core::HRESULT {
    guarded!(match register::unregister_all() {
        Ok(()) => S_OK,
        Err(e) => {
            log::error!("反注册失败: {e}");
            e.into()
        }
    })
}

#[no_mangle]
extern "system" fn DllGetClassObject(
    _rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> windows::core::HRESULT {
    guarded!(unsafe {
        // ClassFactory 只创建 TextService 实例
        IUnknown::from(ClassFactory).query(riid, ppv)
    })
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> windows::core::HRESULT {
    // 见 windows-rs#2472：COM 引用计数回收有限收益，保持常驻
    S_FALSE
}

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        _punkouter: windows_core::Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        unsafe { TextService::create()?.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}
