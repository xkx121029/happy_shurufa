//! 可选 LLM 联想：OpenAI 兼容 chat/completions 接口。
//!
//! 设计约束：HTTP 在独立线程执行，绝不阻塞按键处理线程；
//! 回复通过 mpsc 回传，按键线程以非阻塞 poll 轮询；
//! 响应带 request id，过期结果由调用方丢弃；失败静默降级。

use crate::config::AiConfig;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRequest {
    pub id: u64,
    /// 光标前的上下文（少量字符）
    pub context: String,
    /// 当前正在输入的 token
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiReply {
    pub id: u64,
    pub token: String,
    pub suggestions: Vec<String>,
}

enum Cmd {
    Request(AiRequest),
    Reconfigure(AiConfig),
}

pub struct AiHandle {
    tx: Option<Sender<Cmd>>,
    next_id: u64,
}

impl AiHandle {
    /// 启动后台工作线程。返回（请求句柄，回复接收端）。
    pub fn spawn(config: AiConfig) -> (Self, Receiver<AiReply>) {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let (reply_tx, reply_rx) = mpsc::channel::<AiReply>();
        let _ = std::thread::Builder::new()
            .name("happy-ai".into())
            .spawn(move || worker(config, rx, reply_tx));
        (
            Self {
                tx: Some(tx),
                next_id: 0,
            },
            reply_rx,
        )
    }

    /// 发起联想请求。返回请求 id（用于匹配回复）。未启用时返回 None。
    pub fn request(&mut self, context: String, token: String) -> Option<u64> {
        if token.is_empty() {
            return None;
        }
        self.next_id += 1;
        let id = self.next_id;
        let tx = self.tx.as_ref()?;
        tx.send(Cmd::Request(AiRequest { id, context, token })).ok()?;
        Some(id)
    }

    /// 热更新配置（如启停）。
    pub fn reconfigure(&self, config: AiConfig) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Reconfigure(config));
        }
    }

    /// 按键线程轮询回复（非阻塞）。链路断开时返回 None。
    pub fn poll(rx: &Receiver<AiReply>) -> Option<AiReply> {
        rx.try_recv().ok()
    }
}

fn worker(config: AiConfig, rx: Receiver<Cmd>, reply_tx: Sender<AiReply>) {
    let mut config = config;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Reconfigure(c) => config = c,
            Cmd::Request(req) => {
                if !config.enabled || config.base_url.is_empty() {
                    continue;
                }
                if let Some(reply) = fetch(&config, &req) {
                    let _ = reply_tx.send(reply);
                }
            }
        }
    }
}

fn fetch(config: &AiConfig, req: &AiRequest) -> Option<AiReply> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(config.timeout_ms))
        .build();
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{
            "role": "user",
            "content": format!(
                "你在为程序员输入法做命令补全。以下是编辑器中光标前的内容：\n{}\n用户已输入：{}\n请给出最多5个可能的补全（命令或代码），每行一个，只输出补全内容本身，不要解释。",
                req.context, req.token
            )
        }],
        "max_tokens": 100,
        "temperature": 0.2,
    });
    let response = agent
        .post(&config.base_url)
        .set("Authorization", &format!("Bearer {}", config.api_key))
        .send_json(body)
        .ok()?;
    let json: serde_json::Value = response.into_json().ok()?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()?
        .lines()
        .map(|l| l.trim().trim_start_matches("- ").trim())
        .filter(|l| !l.is_empty())
        .take(5)
        .map(String::from)
        .collect::<Vec<_>>();
    if content.is_empty() {
        return None;
    }
    Some(AiReply {
        id: req.id,
        token: req.token.clone(),
        suggestions: content,
    })
}
