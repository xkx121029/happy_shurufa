//! edit session：一切对文档的读写必须经过 edit session。
//!
//! 模式：实现 #[implement(ITfEditSession)] 的会话结构体，
//! 用 Arc<Mutex<..>> 在会话与调用方之间传递结果，避开 as_impl。

use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use windows::core::BOOL;
use windows::Win32::Foundation::{FALSE, RECT, S_OK};
use windows::Win32::UI::TextServices::{
    GUID_PROP_ATTRIBUTE, ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition,
    ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection, ITfRange, TF_AE_NONE,
    TF_ANCHOR_END, TF_ANCHOR_START, TF_ES_READWRITE, TF_IAS_QUERYONLY, TF_SELECTION,
    TF_SELECTIONSTYLE, TF_ST_CORRECTION,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{Interface, Result, implement};

type Shared<T> = Arc<Mutex<T>>;

fn shared<T>(value: T) -> Shared<T> {
    Arc::new(Mutex::new(value))
}

fn take<T>(slot: &Shared<T>) -> Option<T>
where
    T: Clone,
{
    slot.lock().ok().map(|g| g.clone())
}

fn request(tid: u32, context: &ITfContext, session: &ITfEditSession) -> Result<()> {
    unsafe {
        let result = context.RequestEditSession(tid, session, TF_ES_READWRITE)?;
        if result != S_OK {
            return Err(windows::core::Error::from(result));
        }
    }
    Ok(())
}

/// 在光标处开始 composition（预编辑区），返回 composition 句柄。
pub fn start_composition(
    tid: u32,
    context: &ITfContext,
    composition_sink: &ITfCompositionSink,
) -> Result<ITfComposition> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        composition_sink: ITfCompositionSink,
        out: Shared<Option<ITfComposition>>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let range = self
                    .context
                    .cast::<ITfInsertAtSelection>()?
                    .InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                let context_composition = self.context.cast::<ITfContextComposition>()?;
                let composition =
                    context_composition.StartComposition(ec, &range, &self.composition_sink)?;
                if let Ok(mut slot) = self.out.lock() {
                    *slot = Some(composition);
                }
            }
            Ok(())
        }
    }

    let out = shared(None);
    let session = ITfEditSession::from(Session {
        context: context.clone(),
        composition_sink: composition_sink.clone(),
        out: out.clone(),
    });
    request(tid, context, &session)?;
    take(&out).flatten().ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
}

/// 结束 composition。
pub fn end_composition(tid: u32, context: &ITfContext, composition: &ITfComposition) -> Result<()> {
    #[implement(ITfEditSession)]
    struct Session {
        composition: ITfComposition,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                self.composition.EndComposition(ec)?;
            }
            Ok(())
        }
    }

    let session = ITfEditSession::from(Session {
        composition: composition.clone(),
    });
    request(tid, context, &session)
}

/// 设置 composition / 指定 range 的文本，并把光标移到文本末尾。
pub fn set_text(
    tid: u32,
    context: &ITfContext,
    range: &ITfRange,
    text: &[u16],
    display_attribute: Option<&VARIANT>,
) -> Result<()> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        range: ITfRange,
        text: Vec<u16>,
        display_attribute: Option<VARIANT>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                if let Some(display_attribute) = &self.display_attribute {
                    self.range.SetText(ec, TF_ST_CORRECTION, &self.text)?;
                    let prop = self.context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                    if let Err(e) = prop.SetValue(ec, &self.range, display_attribute) {
                        log::warn!("设置 display attribute 失败: {e}");
                    }
                } else {
                    self.range.SetText(ec, 0, &self.text)?;
                }
                if self.text.is_empty() {
                    return Ok(());
                }
                // 光标移到末尾
                self.range.Collapse(ec, TF_ANCHOR_END)?;
                let selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(self.range.clone())),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_NONE,
                        fInterimChar: FALSE,
                    },
                };
                self.context.SetSelection(ec, &[selection])?;
            }
            Ok(())
        }
    }

    let session = ITfEditSession::from(Session {
        context: context.clone(),
        range: range.clone(),
        text: text.to_vec(),
        display_attribute: display_attribute.cloned(),
    });
    request(tid, context, &session)
}

/// 在光标选区处插入文本。
pub fn insert_text(tid: u32, context: &ITfContext, text: &[u16]) -> Result<()> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        text: Vec<u16>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let range = self
                    .context
                    .cast::<ITfInsertAtSelection>()?
                    .InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                range.SetText(ec, TF_ST_CORRECTION, &self.text)?;
                range.Collapse(ec, TF_ANCHOR_END)?;
                let selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(range.clone())),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_NONE,
                        fInterimChar: FALSE,
                    },
                };
                self.context.SetSelection(ec, &[selection])?;
            }
            Ok(())
        }
    }

    let session = ITfEditSession::from(Session {
        context: context.clone(),
        text: text.to_vec(),
    });
    request(tid, context, &session)
}

/// 将光标前的 `char_count` 个 UTF-16 单元替换为 `text`（用于命令补全）。
/// 注意：仅支持 ASCII token（编程场景足够）。
pub fn replace_before_caret(
    tid: u32,
    context: &ITfContext,
    char_count: usize,
    text: &[u16],
) -> Result<()> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        char_count: i32,
        text: Vec<u16>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let range = self
                    .context
                    .cast::<ITfInsertAtSelection>()?
                    .InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                // 折叠到选区起点，再把 start 向左移动 N 个字符
                range.Collapse(ec, TF_ANCHOR_START)?;
                let mut actual = 0i32;
                range.ShiftStart(ec, -self.char_count, &mut actual, std::ptr::null())?;
                range.SetText(ec, 0, &self.text)?;
                range.Collapse(ec, TF_ANCHOR_END)?;
                let selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(range.clone())),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_NONE,
                        fInterimChar: FALSE,
                    },
                };
                self.context.SetSelection(ec, &[selection])?;
            }
            Ok(())
        }
    }

    let session = ITfEditSession::from(Session {
        context: context.clone(),
        char_count: char_count as i32,
        text: text.to_vec(),
    });
    request(tid, context, &session)
}

/// 读取光标前最多 `max_units` 个 UTF-16 单元的文本（READ 会话）。
pub fn get_preceding_text(tid: u32, context: &ITfContext, max_units: usize) -> Result<String> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        max_units: usize,
        out: Shared<String>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let range = self
                    .context
                    .cast::<ITfInsertAtSelection>()?
                    .InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                range.Collapse(ec, TF_ANCHOR_START)?;
                let mut actual = 0i32;
                range.ShiftStart(ec, -(self.max_units as i32), &mut actual, std::ptr::null())?;
                let mut buf = [0u16; 128];
                let mut fetched = 0u32;
                range.GetText(ec, 0, &mut buf, &mut fetched)?;
                if let Ok(mut slot) = self.out.lock() {
                    *slot = String::from_utf16_lossy(&buf[..fetched as usize]);
                }
            }
            Ok(())
        }
    }

    let out = shared(String::new());
    let session = ITfEditSession::from(Session {
        context: context.clone(),
        max_units,
        out: out.clone(),
    });
    // READ 权限即可；同步请求在按键上下文中是安全的
    unsafe {
        let result = context.RequestEditSession(
            tid,
            &session,
            windows::Win32::UI::TextServices::TF_ES_READ | windows::Win32::UI::TextServices::TF_ES_SYNC,
        )?;
        if result != S_OK {
            return Err(windows::core::Error::from(result));
        }
    }
    Ok(take(&out).unwrap_or_default())
}

/// composition 范围的屏幕坐标（光标位置，物理像素）。
pub fn get_pos(tid: u32, context: &ITfContext, range: &ITfRange) -> Result<(i32, i32)> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        range: ITfRange,
        out: Shared<(i32, i32)>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let mut rect = RECT::default();
                let mut clipped = BOOL::default();
                let view = self.context.GetActiveView()?;
                view.GetTextExt(ec, &self.range, &mut rect, &mut clipped)?;
                if let Ok(mut slot) = self.out.lock() {
                    *slot = (rect.left, rect.bottom);
                }
            }
            Ok(())
        }
    }

    let out = shared((0, 0));
    let session = ITfEditSession::from(Session {
        context: context.clone(),
        range: range.clone(),
        out: out.clone(),
    });
    request(tid, context, &session)?;
    Ok(take(&out).unwrap_or((0, 0)))
}

/// 光标前文本（不带 selection 依赖，用于 EN 模式 token 探测）。
/// 供键事件路径调用；失败返回空串。
pub fn preceding_text(tid: u32, context: &ITfContext) -> String {
    get_preceding_text(tid, context, 96).unwrap_or_default()
}

/// 光标（选区）的屏幕坐标，无需 composition。失败返回 None。
pub fn caret_pos(tid: u32, context: &ITfContext) -> Option<(i32, i32)> {
    #[implement(ITfEditSession)]
    struct Session {
        context: ITfContext,
        out: Shared<(i32, i32)>,
    }

    impl ITfEditSession_Impl for Session_Impl {
        fn DoEditSession(&self, ec: u32) -> Result<()> {
            unsafe {
                let range = self
                    .context
                    .cast::<ITfInsertAtSelection>()?
                    .InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                let mut rect = RECT::default();
                let mut clipped = BOOL::default();
                let view = self.context.GetActiveView()?;
                view.GetTextExt(ec, &range, &mut rect, &mut clipped)?;
                if let Ok(mut slot) = self.out.lock() {
                    *slot = (rect.left, rect.bottom);
                }
            }
            Ok(())
        }
    }

    let out = shared((0, 0));
    let session = ITfEditSession::from(Session {
        context: context.clone(),
        out: out.clone(),
    });
    unsafe {
        let result = context.RequestEditSession(
            tid,
            &session,
            windows::Win32::UI::TextServices::TF_ES_READ | windows::Win32::UI::TextServices::TF_ES_SYNC,
        )
        .ok()?;
        if result != S_OK {
            return None;
        }
    }
    let (x, y) = take(&out)?;
    if x <= 0 && y <= 0 {
        None
    } else {
        Some((x, y))
    }
}
