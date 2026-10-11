//! ITfDisplayAttributeProvider：composition 下划线样式。
//!
//! 注意：Windows 不会经类工厂获取该接口，而是直接 cast ITfTextInputProcessor，
//! 因此必须在 #[implement] 列表中包含本接口。

use windows::Win32::Foundation::{E_INVALIDARG, E_NOTIMPL};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfDisplayAttributeInfo,
    ITfDisplayAttributeInfo_Impl, ITfDisplayAttributeProvider_Impl, TF_ATTR_INPUT, TF_DA_COLOR,
    TF_DISPLAYATTRIBUTE, TF_LS_SOLID,
};
use windows::core::{BSTR, GUID, Result, implement};

use super::TextService_Impl;
use crate::global::DISPLAY_ATTR_ID;

#[allow(non_snake_case)]
impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumDisplayAttributeInfo::create())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if unsafe { *guid } == DISPLAY_ATTR_ID {
            Ok(DisplayAttributeInfo::create())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}

/// 只有一个属性的枚举器。
#[implement(IEnumTfDisplayAttributeInfo)]
struct EnumDisplayAttributeInfo {
    enumerated: std::sync::atomic::AtomicBool,
}

impl EnumDisplayAttributeInfo {
    fn create() -> IEnumTfDisplayAttributeInfo {
        IEnumTfDisplayAttributeInfo::from(Self {
            enumerated: std::sync::atomic::AtomicBool::new(false),
        })
    }
}

#[allow(non_snake_case)]
impl IEnumTfDisplayAttributeInfo_Impl for EnumDisplayAttributeInfo_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Err(E_NOTIMPL.into())
    }

    fn Next(
        &self,
        _count: u32,
        info: *mut Option<ITfDisplayAttributeInfo>,
        fetched: *mut u32,
    ) -> Result<()> {
        unsafe {
            if !self.enumerated.swap(true, std::sync::atomic::Ordering::Relaxed) {
                *info = Some(DisplayAttributeInfo::create());
                *fetched = 1;
            } else {
                *fetched = 0;
            }
        }
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        self.enumerated
            .store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn Skip(&self, count: u32) -> Result<()> {
        if count > 0 {
            self.enumerated
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }
}

/// 唯一的 display attribute：实线下划线。
#[implement(ITfDisplayAttributeInfo)]
#[derive(Default)]
struct DisplayAttributeInfo;

impl DisplayAttributeInfo {
    fn create() -> ITfDisplayAttributeInfo {
        ITfDisplayAttributeInfo::from(Self)
    }
}

#[allow(non_snake_case)]
impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(DISPLAY_ATTR_ID)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Happy 输入法预编辑"))
    }

    fn GetAttributeInfo(&self, attr: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        unsafe {
            *attr = TF_DISPLAYATTRIBUTE {
                crText: TF_DA_COLOR::default(),
                crBk: TF_DA_COLOR::default(),
                crLine: TF_DA_COLOR::default(),
                lsStyle: TF_LS_SOLID,
                fBoldLine: false.into(),
                bAttr: TF_ATTR_INPUT,
            };
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _attr: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Reset(&self) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
}
