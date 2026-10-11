//! ITfThreadMgrEventSink：焦点变化时中止 composition 并刷新进程模式。

use windows::Win32::UI::TextServices::{ITfDocumentMgr, ITfThreadMgrEventSink_Impl};
use windows::core::Result;

use super::TextService_Impl;

#[allow(non_snake_case)]
impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _pdim: windows_core::Ref<'_, ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _pdim: windows_core::Ref<'_, ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    /// 文档焦点切换：中止一切进行中的状态，刷新进程 → 模式。
    fn OnSetFocus(
        &self,
        _focus: windows_core::Ref<'_, ITfDocumentMgr>,
        _prevfocus: windows_core::Ref<'_, ITfDocumentMgr>,
    ) -> Result<()> {
        let mut inner = self.try_write()?;
        if inner.composition.is_some() {
            // 应用已终止 composition 的情况下只清理状态
            inner.clear_compose_state();
            inner.hide_candidates();
        }
        inner.clear_cmd_state();
        inner.refresh_mode();
        Ok(())
    }

    fn OnPushContext(&self, _pic: windows_core::Ref<'_, windows::Win32::UI::TextServices::ITfContext>) -> Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _pic: windows_core::Ref<'_, windows::Win32::UI::TextServices::ITfContext>) -> Result<()> {
        Ok(())
    }
}
