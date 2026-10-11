//! composition 状态流转：拼音预编辑、候选提交、命令补全提交。

use windows::Win32::UI::TextServices::{ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl};
use windows::core::{Interface, Result};

use super::edit_session;
use super::{TextServiceInner, TextService_Impl};
use crate::window::DisplayItem;

impl TextServiceInner {
    // ------------------------------------------------------------------
    //  中文模式：composition 生命周期
    // ------------------------------------------------------------------

    pub fn start_compose(&mut self) -> Result<()> {
        let context = self.context()?.clone();
        let interface = self.interface()?.cast::<ITfCompositionSink>()?;
        let composition = edit_session::start_composition(self.tid, &context, &interface)?;
        self.composition = Some(composition);
        Ok(())
    }

    /// 结束 composition（保留现有文本）并清空状态。
    pub fn end_compose(&mut self) -> Result<()> {
        if let (Some(context), Some(composition)) =
            (self.context.as_ref().cloned(), self.composition.take())
        {
            let _ = edit_session::end_composition(self.tid, &context, &composition);
        }
        self.clear_compose_state();
        self.hide_candidates();
        Ok(())
    }

    fn composition_range(&self) -> Result<windows::Win32::UI::TextServices::ITfRange> {
        let composition = self
            .composition
            .as_ref()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        unsafe { composition.GetRange() }
    }

    /// 更新预编辑文本（候选串 + 剩余拼音）。
    fn update_preedit(&mut self) -> Result<()> {
        let preedit = format!("{}{}", self.selected, self.spelling);
        let context = self.context()?.clone();
        let range = self.composition_range()?;
        let text: Vec<u16> = preedit.encode_utf16().collect();
        let attr = self.display_attribute.clone();
        edit_session::set_text(self.tid, &context, &range, &text, attr.as_ref())
    }

    /// 更新候选窗内容与位置。
    fn update_candidates(&mut self) -> Result<()> {
        self.assure_candidates();
        let items: Vec<DisplayItem> = self
            .suggestions
            .iter()
            .enumerate()
            .map(|(_i, s)| DisplayItem {
                label: s.clone(),
                desc: String::new(),
            })
            .collect();
        if items.is_empty() {
            self.hide_candidates();
            return Ok(());
        }
        let window = self
            .candidates
            .as_ref()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        match self.caret_pos() {
            Some((x, y)) => window.show(&items, x + 2, y + 4)?,
            None => window.show(&items, 200, 200)?,
        }
        Ok(())
    }

    fn caret_pos(&self) -> Option<(i32, i32)> {
        let context = self.context.as_ref()?;
        if let Some(composition) = self.composition.as_ref() {
            let range = unsafe { composition.GetRange().ok()? };
            let (x, y) = edit_session::get_pos(self.tid, context, &range).ok()?;
            if x > 0 || y > 0 {
                return Some((x, y));
            }
        }
        edit_session::caret_pos(self.tid, context)
    }

    // ------------------------------------------------------------------
    //  中文模式：按键动作
    // ------------------------------------------------------------------

    /// 追加拼音字母。
    pub fn push_letter(&mut self, ch: char) -> Result<()> {
        if self.composition.is_none() {
            self.start_compose()?;
        }
        self.spelling.push(ch);
        self.refresh_suggestions();
        self.update_preedit()?;
        self.update_candidates()
    }

    /// 回删一个拼音字母；空了则静默结束。
    pub fn pop_letter(&mut self) -> Result<()> {
        self.spelling.pop();
        if self.spelling.is_empty() && self.selected.is_empty() {
            return self.cancel_compose();
        }
        self.refresh_suggestions();
        self.update_preedit()?;
        self.update_candidates()
    }

    fn refresh_suggestions(&mut self) {
        let limit = self.config.candidate_count;
        self.suggestions = happy_core::pinyin::PinyinEngine::suggest_with(&self.spelling, limit);
    }

    /// 数字/空格选择候选。
    pub fn select_candidate(&mut self, index: usize) -> Result<()> {
        let Some(word) = self.suggestions.get(index).cloned() else {
            return Ok(());
        };
        self.selected.push_str(&word);
        self.spelling.clear();
        self.suggestions.clear();
        if self.spelling.is_empty() {
            return self.commit_selected();
        }
        self.update_preedit()?;
        self.update_candidates()
    }

    /// 已选定的候选上屏并结束 composition。
    pub fn commit_selected(&mut self) -> Result<()> {
        let text = self.selected.clone();
        self.selected.clear();
        let context = self.context()?.clone();
        let range = self.composition_range()?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        edit_session::set_text(self.tid, &context, &range, &utf16, None)?;
        self.end_compose()
    }

    /// 空格：有候选选第一个，否则原始字母 + 空格上屏。
    pub fn commit_with_space(&mut self) -> Result<()> {
        if !self.suggestions.is_empty() {
            return self.select_candidate(0);
        }
        let text = format!("{}{} ", self.selected, self.spelling);
        self.selected.clear();
        self.spelling.clear();
        let context = self.context()?.clone();
        let range = self.composition_range()?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        edit_session::set_text(self.tid, &context, &range, &utf16, None)?;
        self.end_compose()
    }

    /// 回车：原始字母原样上屏。
    pub fn release_raw(&mut self) -> Result<()> {
        let text = format!("{}{}", self.selected, self.spelling);
        self.selected.clear();
        self.spelling.clear();
        if text.is_empty() {
            return self.cancel_compose();
        }
        let context = self.context()?.clone();
        let range = self.composition_range()?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        edit_session::set_text(self.tid, &context, &range, &utf16, None)?;
        self.end_compose()
    }

    /// 取消：清空预编辑。
    pub fn cancel_compose(&mut self) -> Result<()> {
        if self.composition.is_some() {
            let context = self.context()?.clone();
            let range = self.composition_range()?;
            edit_session::set_text(self.tid, &context, &range, &[], None)?;
        }
        self.end_compose()
    }

    /// 中英切换时调用：把原始字母放行上屏再切模式。
    pub fn flush_and_toggle(&mut self) -> Result<()> {
        if self.composition.is_some() {
            let _ = self.release_raw();
        }
        self.hide_candidates();
        self.clear_cmd_state();
        let _ = self.mode.toggle();
        self.save_profile();
        log::info!("切换模式: {:?}", self.mode.current);
        Ok(())
    }

    // ------------------------------------------------------------------
    //  英文模式：命令联想
    // ------------------------------------------------------------------

    /// 字母键入后刷新命令联想。`typed` 为本次键入字符（可能为空）。
    pub fn refresh_cmd_suggest(&mut self, typed: char) -> Result<()> {
        let Some(context) = self.context.as_ref().cloned() else {
            return Ok(());
        };
        let preceding = edit_session::preceding_text(self.tid, &context);
        let mut token = trailing_token(&preceding);
        if typed.is_ascii_alphanumeric() {
            token.push(typed);
        }
        if token.is_empty() || !self.config.command_suggest {
            self.clear_cmd_state();
            self.hide_candidates();
            return Ok(());
        }
        let limit = self.config.candidate_count;
        self.cmd_hits = happy_core::lexicon::suggest(&token, &self.freq, limit);
        self.cmd_token = token.clone();
        let items: Vec<DisplayItem> = self
            .cmd_hits
            .iter()
            .map(|h| DisplayItem {
                label: h.text.clone(),
                desc: h.desc.clone(),
            })
            .collect();
        if items.is_empty() {
            self.hide_candidates();
            return Ok(());
        }
        self.assure_candidates();
        let window = self
            .candidates
            .as_ref()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        let pos = self.caret_pos().unwrap_or((200, 200));
        window.show(&items, pos.0 + 2, pos.1 + 4)?;
        self.cmd_suggests = items;
        // AI 联想（可选，异步）
        self.maybe_request_ai(&preceding, &token);
        Ok(())
    }

    fn maybe_request_ai(&mut self, preceding: &str, token: &str) {
        let ai_cfg = self.config.ai.clone();
        if !ai_cfg.enabled || token.chars().count() < ai_cfg.trigger_len {
            return;
        }
        if self.ai.is_none() {
            let (handle, rx) = happy_core::ai::AiHandle::spawn(ai_cfg);
            self.ai = Some(handle);
            self.ai_rx = Some(rx);
        }
        let context: String = preceding
            .chars()
            .rev()
            .take(120)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if let Some(handle) = &mut self.ai {
            self.pending_ai = handle.request(context, token.to_string());
        }
    }

    /// 轮询 AI 回复（按键线程调用，非阻塞）。
    pub fn poll_ai(&mut self) {
        let Some(rx) = &self.ai_rx else { return };
        if let Some(reply) = happy_core::ai::AiHandle::poll(rx) {
            if self.pending_ai == Some(reply.id) && reply.token == self.cmd_token {
                // 追加 AI 建议到窗口
                let limit = self.config.candidate_count;
                let mut items = self.cmd_suggests.clone();
                for s in reply.suggestions {
                    if items.len() >= limit {
                        break;
                    }
                    if items.iter().any(|i| i.label == s) {
                        continue;
                    }
                    items.push(DisplayItem {
                        label: s,
                        desc: "AI".into(),
                    });
                }
                if let Some(window) = &self.candidates {
                    let pos = self.caret_pos().unwrap_or((200, 200));
                    let _ = window.show(&items, pos.0 + 2, pos.1 + 4);
                }
            }
        }
    }

    /// 提交第一条命令联想（Tab）。
    pub fn commit_cmd(&mut self, index: usize) -> Result<()> {
        let Some(hit) = self.cmd_hits.get(index).cloned() else {
            return Ok(());
        };
        let token_len = self.cmd_token.chars().count();
        let context = self.context()?.clone();
        let utf16: Vec<u16> = hit.text.encode_utf16().collect();
        edit_session::replace_before_caret(self.tid, &context, token_len, &utf16)?;
        self.freq.bump(&hit.text);
        self.save_profile();
        self.clear_cmd_state();
        self.hide_candidates();
        Ok(())
    }
}

/// 提取文本末尾的 [A-Za-z0-9_] 连续串。
pub fn trailing_token(text: &str) -> String {
    text.chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

// ----------------------------------------------------------------------

#[allow(non_snake_case)]
impl ITfCompositionSink_Impl for TextService_Impl {
    /// 应用主动终止 composition（如焦点变化）：保留文档中已有文本，仅清内部状态。
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _composition: windows_core::Ref<'_, ITfComposition>,
    ) -> Result<()> {
        log::debug!("OnCompositionTerminated");
        let mut inner = self.try_write()?;
        if inner.composition.is_some() {
            inner.composition = None;
            inner.spelling.clear();
            inner.selected.clear();
            inner.suggestions.clear();
            inner.hide_candidates();
        }
        Ok(())
    }
}
