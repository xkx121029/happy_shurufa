//! TSF 文本服务：接口装配与共享状态。

pub mod composition;
pub mod display_attribute_provider;
pub mod edit_session;
pub mod key_event_sink;
pub mod text_input_processor;
pub mod thread_mgr_event_sink;

use std::sync::RwLock;
use std::sync::mpsc::Receiver;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use happy_core::ai::{AiHandle, AiReply};
use happy_core::config::Config;
use happy_core::freq::FreqMap;
use happy_core::mode::{ModeState, Mode};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfDisplayAttributeProvider,
    ITfKeyEventSink, ITfTextInputProcessor, ITfTextInputProcessorEx, ITfThreadMgr,
    ITfThreadMgrEventSink,
};
use windows_core::{implement, AsImpl};

use crate::profile;
use crate::window::{CandidateWindow, DisplayItem};

/// TSF 文本服务本体：所有事件 sink 挂在同一个 COM 对象上。
#[implement(
    ITfTextInputProcessor,
    ITfTextInputProcessorEx,
    ITfThreadMgrEventSink,
    ITfKeyEventSink,
    ITfCompositionSink,
    ITfDisplayAttributeProvider
)]
pub struct TextService {
    pub inner: RwLock<TextServiceInner>,
}

pub struct TextServiceInner {
    // TSF 会话
    pub tid: u32,
    pub thread_mgr: Option<ITfThreadMgr>,
    pub context: Option<ITfContext>,
    pub cookie: Option<u32>,
    pub interface: Option<ITfTextInputProcessor>,
    pub display_attribute: Option<VARIANT>,
    // 引擎与配置
    pub config: Config,
    pub mode: ModeState,
    pub freq: FreqMap,
    // 中文 composition 状态
    pub composition: Option<ITfComposition>,
    pub spelling: String,   // 已输入拼音字母
    pub selected: String,   // 已连续选定的候选
    pub suggestions: Vec<String>,
    // 英文模式命令联想状态
    pub cmd_suggests: Vec<DisplayItem>, // 窗口条目（命令 + AI 合并）
    pub cmd_hits: Vec<happy_core::lexicon::CommandHit>, // 命令命中（可提交）
    pub cmd_token: String,              // 关联的输入 token（ASCII）
    pub pending_ai: Option<u64>,
    pub ai: Option<AiHandle>,
    pub ai_rx: Option<Receiver<AiReply>>,
    // UI
    pub candidates: Option<CandidateWindow>,
}

impl TextService {
    pub fn create() -> windows::core::Result<ITfTextInputProcessor> {
        let config = Config::load();
        let mut mode = ModeState::new(config.mode_learning);
        mode.memory = profile::load_modes();
        let inner = TextServiceInner {
            tid: 0,
            thread_mgr: None,
            context: None,
            cookie: None,
            interface: None,
            display_attribute: None,
            config,
            mode,
            freq: profile::load_freq(),
            composition: None,
            spelling: String::new(),
            selected: String::new(),
            suggestions: Vec::new(),
            cmd_suggests: Vec::new(),
            cmd_hits: Vec::new(),
            cmd_token: String::new(),
            pending_ai: None,
            ai: None,
            ai_rx: None,
            candidates: None,
        };
        let service = Self {
            inner: RwLock::new(inner),
        };
        let interface = ITfTextInputProcessor::from(service);
        // 把自身 COM 接口注回内部状态（Activate 时用于 cast 出各事件 sink）
        unsafe {
            let this: &TextService = interface.as_impl();
            if let Ok(mut inner) = this.inner.write() {
                inner.interface = Some(interface.clone());
            }
        }
        Ok(interface)
    }

    pub fn write(&self) -> windows::core::Result<RwLockWriteGuard<'_, TextServiceInner>> {
        self.inner.write().map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }

    /// 焦点切换等重入场景使用，避免死锁。
    pub fn try_write(&self) -> windows::core::Result<RwLockWriteGuard<'_, TextServiceInner>> {
        self.inner
            .try_write()
            .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }

    #[allow(dead_code)]
    pub fn read(&self) -> windows::core::Result<RwLockReadGuard<'_, TextServiceInner>> {
        self.inner.read().map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }
}

impl TextServiceInner {
    pub fn thread_mgr(&self) -> windows::core::Result<&ITfThreadMgr> {
        self.thread_mgr
            .as_ref()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }

    pub fn context(&self) -> windows::core::Result<&ITfContext> {
        self.context
            .as_ref()
            .ok_or(windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }

    /// 组合窗口条目：命令命中 + AI 回复合并。
    pub fn build_cmd_items(&self) -> Vec<DisplayItem> {
        let mut items: Vec<DisplayItem> = self
            .cmd_hits
            .iter()
            .map(|h| DisplayItem {
                label: h.text.clone(),
                desc: h.desc.clone(),
            })
            .collect();
        items.truncate(self.config.candidate_count);
        items
    }

    /// 持久化模式记忆与频率。
    pub fn save_profile(&self) {
        profile::save_modes(&self.mode.memory);
        profile::save_freq(&self.freq);
    }

    /// 当前模式是否中文。
    pub fn is_chinese(&self) -> bool {
        self.mode.current == Mode::Chinese
    }

    /// 前台进程变化后刷新模式。
    pub fn refresh_mode(&mut self) {
        let exe = crate::context::foreground_process().unwrap_or_default();
        if self.config.builtin_rules || !self.mode.memory.manual.is_empty() {
            self.mode.on_process_change(&exe);
        }
        log::info!("进程: {exe:?}, 模式: {:?}", self.mode.current);
    }

    /// 确保候选窗存在（创建失败时下次重建）。
    pub fn assure_candidates(&mut self) {
        if self.candidates.is_none() {
            match CandidateWindow::create() {
                Ok(w) => self.candidates = Some(w),
                Err(e) => log::error!("候选窗创建失败: {e}"),
            }
        }
    }

    pub fn hide_candidates(&mut self) {
        if let Some(w) = &self.candidates {
            w.hide();
        }
    }

    pub fn clear_compose_state(&mut self) {
        self.composition = None;
        self.spelling.clear();
        self.selected.clear();
        self.suggestions.clear();
        self.cmd_suggests.clear();
        self.cmd_hits.clear();
        self.cmd_token.clear();
        self.pending_ai = None;
    }

    pub fn clear_cmd_state(&mut self) {
        self.cmd_suggests.clear();
        self.cmd_hits.clear();
        self.cmd_token.clear();
        self.pending_ai = None;
    }
}
