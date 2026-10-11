//! ITfKeyEventSink：输入解析与分发（本输入法的中枢）。

use windows::core::BOOL;
use windows::Win32::Foundation::{FALSE, LPARAM, TRUE, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardState, ToUnicodeEx, VK_BACK, VK_ESCAPE, VK_RETURN, VK_SPACE,
    VK_TAB,
};
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink_Impl};
use windows::core::{GUID, Result};

use super::{TextServiceInner, TextService_Impl};
use crate::global::zh_cn_us_hkl;

#[allow(non_snake_case)]
impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 返回 true 表示"之后可能"拦截此键；客户端可能不再调用 OnKeyDown。
    fn OnTestKeyDown(&self, _context: windows_core::Ref<'_, ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let mut inner = self.write()?;
        Ok(inner.test_key(wparam.0))
    }

    /// 返回 true 表示按键已消费。
    fn OnKeyDown(&self, context: windows_core::Ref<'_, ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let mut inner = self.write()?;
        if let Some(ctx) = context.cloned() {
            inner.context = Some(ctx);
        }
        let scancode = ((lparam.0 as usize) >> 16) & 0xFF;
        inner.handle_key(wparam.0, scancode as u32)
    }

    fn OnTestKeyUp(&self, _context: windows_core::Ref<'_, ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _context: windows_core::Ref<'_, ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    /// PreservedKey 触发（Shift 单击 = 中英切换）。
    fn OnPreservedKey(&self, _context: windows_core::Ref<'_, ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        let guid = unsafe { rguid.as_ref().copied() };
        if guid == Some(crate::global::SHIFT_TOGGLE_KEY) {
            let mut inner = self.write()?;
            inner.flush_and_toggle()?;
            return Ok(TRUE);
        }
        Ok(FALSE)
    }

    fn OnSetFocus(&self, _foreground: BOOL) -> Result<()> {
        Ok(())
    }
}

impl TextServiceInner {
    /// OnTestKeyDown 决策：是否请求拦截该键。
    fn test_key(&mut self, vk: usize) -> BOOL {
        match vk {
            v if v == VK_BACK.0 as usize || v == VK_ESCAPE.0 as usize => {
                if self.composition.is_some() || self.has_suggestions() {
                    TRUE
                } else {
                    FALSE
                }
            }
            v if v == VK_TAB.0 as usize => {
                if self.has_cmd_suggestions() {
                    TRUE
                } else {
                    FALSE
                }
            }
            v if v == VK_RETURN.0 as usize || v == VK_SPACE.0 as usize => {
                if self.composition.is_some() {
                    TRUE
                } else {
                    // EN 模式空格/回车清掉联想窗后放行
                    if self.has_cmd_suggestions() {
                        self.clear_cmd_state();
                        self.hide_candidates();
                    }
                    FALSE
                }
            }
            // 可打印键（字母/数字/标点）
            _ => {
                if parse_char(vk as u32, 0).is_none() {
                    return FALSE;
                }
                if self.is_chinese() {
                    // 中文模式：字母进入拼音，标点/数字按半角处理
                    TRUE
                } else {
                    // 英文模式：放行；仅命令联想数字选择时拦截
                    let is_digit = parse_char(vk as u32, 0)
                        .and_then(|c| c.to_digit(10))
                        .is_some();
                    if is_digit && self.has_cmd_suggestions() {
                        TRUE
                    } else {
                        FALSE
                    }
                }
            }
        }
    }

    /// OnKeyDown 实际处理，返回是否消费。
    fn handle_key(&mut self, vk: usize, scancode: u32) -> Result<BOOL> {
        // AI 回复轮询（非阻塞）
        self.poll_ai();

        if vk == VK_ESCAPE.0 as usize {
            if self.composition.is_some() {
                self.cancel_compose()?;
                return Ok(TRUE);
            }
            if self.has_suggestions() {
                self.clear_cmd_state();
                self.hide_candidates();
                return Ok(TRUE);
            }
            return Ok(FALSE);
        }

        if vk == VK_BACK.0 as usize {
            if self.composition.is_some() {
                self.pop_letter()?;
                return Ok(TRUE);
            }
            return Ok(FALSE);
        }

        if vk == VK_TAB.0 as usize {
            if self.has_cmd_suggestions() {
                self.commit_cmd(0)?;
                return Ok(TRUE);
            }
            return Ok(FALSE);
        }

        if vk == VK_RETURN.0 as usize {
            if self.composition.is_some() {
                self.release_raw()?;
                return Ok(TRUE);
            }
            return Ok(FALSE);
        }

        if vk == VK_SPACE.0 as usize {
            if self.composition.is_some() {
                self.commit_with_space()?;
                return Ok(TRUE);
            }
            return Ok(FALSE);
        }

        let Some(ch) = parse_char(vk as u32, scancode) else {
            return Ok(FALSE);
        };

        if self.is_chinese() {
            if ch.is_ascii_alphabetic() {
                // 字母 → 拼音 composition
                self.push_letter(ch.to_ascii_lowercase())?;
                return Ok(TRUE);
            }
            if self.composition.is_some() {
                if ch.is_ascii_digit() {
                    let idx = ch.to_digit(10).unwrap_or(0) as usize;
                    if idx >= 1 && idx <= self.suggestions.len() {
                        self.select_candidate(idx - 1)?;
                        return Ok(TRUE);
                    }
                    return Ok(FALSE);
                }
                // 标点：先提交当前候选/原始字母，再输出符号
                if !self.suggestions.is_empty() {
                    self.select_candidate(0)?;
                } else {
                    self.release_raw()?;
                }
            }
            // 标点/数字直出（半角映射）
            self.insert_punct(ch)?;
            Ok(TRUE)
        } else {
            // 英文模式
            if ch.is_ascii_digit() && self.has_cmd_suggestions() {
                let idx = ch.to_digit(10).unwrap_or(0) as usize;
                if idx >= 1 && self.cmd_hits.len() >= idx {
                    self.commit_cmd(idx - 1)?;
                    return Ok(TRUE);
                }
            }
            // 放行，但刷新命令联想
            self.refresh_cmd_suggest(ch)?;
            Ok(FALSE)
        }
    }

    fn insert_punct(&mut self, ch: char) -> Result<()> {
        let out = if self.config.halfwidth_punct {
            happy_core::punct::normalize(ch)
        } else {
            ch
        };
        let context = self.context()?.clone();
        let mut buf = [0u16; 4];
        let n = out.encode_utf16(&mut buf).len();
        super::edit_session::insert_text(self.tid, &context, &buf[..n])
    }

    fn has_suggestions(&self) -> bool {
        !self.cmd_suggests.is_empty() || !self.suggestions.is_empty()
    }

    fn has_cmd_suggestions(&self) -> bool {
        !self.cmd_suggests.is_empty()
    }
}

/// VK → 字符（美式键盘布局保证字母/标点为 ASCII）。
fn parse_char(vk: u32, scancode: u32) -> Option<char> {
    match vk {
        0x00..=0x1F | 0x7F => None,
        _ => unsafe {
            let mut keyboard_state = [0u8; 256];
            if GetKeyboardState(&mut keyboard_state).is_err() {
                return None;
            }
            let mut buf = [0u16; 8];
            let ret = ToUnicodeEx(
                vk,
                scancode,
                &keyboard_state,
                &mut buf,
                0,
                Some(zh_cn_us_hkl()),
            );
            if ret <= 0 {
                return None;
            }
            char::from_u32(buf[0] as u32)
        },
    }
}
