//! ITfTextInputProcessor：激活/停用，装配事件 sink 与 Shift 切换 PreservedKey。

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, ITfCategoryMgr, ITfKeyEventSink, ITfKeystrokeMgr, ITfSource,
    ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfTextInputProcessorEx_Impl, ITfThreadMgr,
    ITfThreadMgrEventSink, TF_MOD_ON_KEYUP, TF_PRESERVEDKEY,
};
use windows::core::{Interface, Result};

use super::{TextServiceInner, TextService_Impl};
use crate::global::{DISPLAY_ATTR_ID, SHIFT_TOGGLE_KEY};

#[allow(non_snake_case)]
impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, thread_mgr: windows_core::Ref<'_, ITfThreadMgr>, tid: u32) -> Result<()> {
        log::info!("Activate(tid={tid})");
        let mut inner = self.write()?;
        let thread_mgr = thread_mgr.ok()?.clone();
        inner.tid = tid;
        inner.thread_mgr = Some(thread_mgr.clone());

        unsafe {
            // 1. 按键事件 sink
            thread_mgr.cast::<ITfKeystrokeMgr>()?.AdviseKeyEventSink(
                tid,
                &inner.interface()?.cast::<ITfKeyEventSink>()?,
                true,
            )?;
            // 2. 线程管理器事件 sink（焦点变化）
            inner.cookie = Some(thread_mgr.cast::<ITfSource>()?.AdviseSink(
                &ITfThreadMgrEventSink::IID,
                &inner.interface()?.cast::<ITfThreadMgrEventSink>()?,
            )?);
            // 3. Shift 单击切换中英（key-up 触发，按键间隔无效）
            thread_mgr.cast::<ITfKeystrokeMgr>()?.PreserveKey(
                tid,
                &SHIFT_TOGGLE_KEY,
                &TF_PRESERVEDKEY {
                    uVKey: VK_SHIFT.0 as u32,
                    uModifiers: TF_MOD_ON_KEYUP,
                },
                &[],
            )?;
            // 4. composition 下划线 display attribute
            if inner.display_attribute.is_none() {
                let category_mgr: ITfCategoryMgr =
                    CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
                let guid_atom = category_mgr.RegisterGUID(&DISPLAY_ATTR_ID)?;
                inner.display_attribute = Some(VARIANT::from(guid_atom as i32));
            }
        }

        // 5. 探测进程并确定中英模式
        inner.refresh_mode();
        // 6. 候选窗
        inner.assure_candidates();
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        log::info!("Deactivate");
        let mut inner = self.write()?;
        let cookie = inner.cookie.take();
        if let Ok(thread_mgr) = inner.thread_mgr().cloned() {
            unsafe {
                let keystroke = thread_mgr.cast::<ITfKeystrokeMgr>();
                if let Ok(keystroke) = keystroke {
                    let _ = keystroke.UnadviseKeyEventSink(inner.tid);
                    let _ = keystroke.UnpreserveKey(&SHIFT_TOGGLE_KEY, &TF_PRESERVEDKEY {
                        uVKey: VK_SHIFT.0 as u32,
                        uModifiers: TF_MOD_ON_KEYUP,
                    });
                }
                if let Some(cookie) = cookie {
                    if let Ok(source) = thread_mgr.cast::<ITfSource>() {
                        let _ = source.UnadviseSink(cookie);
                    }
                }
            }
        }
        if let Some(window) = inner.candidates.take() {
            let _ = window.destroy();
        }
        inner.clear_compose_state();
        inner.save_profile();
        inner.thread_mgr = None;
        Ok(())
    }
}

#[allow(non_snake_case)]
impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(
        &self,
        thread_mgr: windows_core::Ref<'_, ITfThreadMgr>,
        tid: u32,
        _dwflags: u32,
    ) -> Result<()> {
        self.Activate(thread_mgr, tid)
    }
}

impl TextServiceInner {
    /// 取回自身 COM 接口（Activate 时装配 sink 用）。
    pub fn interface(
        &self,
    ) -> windows::core::Result<ITfTextInputProcessor> {
        self.interface
            .as_ref()
            .cloned()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }
}
