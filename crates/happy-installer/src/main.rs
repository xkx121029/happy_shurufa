//! Happy 输入法安装/卸载器
//!
//! 用法：
//!   happy-installer install    安装（拷贝 DLL 到 Program Files 并注册）
//!   happy-installer uninstall  卸载（反注册并删除文件）
//!
//! 需要管理员权限（写 HKLM 与 Program Files；失败时回退 HKCU 与用户目录）。

use std::process::Command;

const INSTALL_DIR: &str = r"%ProgramFiles%\happy_shurufa";
const DLL_NAME: &str = "happy_ime.dll";

fn main() {
    let action = std::env::args().nth(1).unwrap_or_default();
    let result = match action.as_str() {
        "install" => install(),
        "uninstall" => uninstall(),
        _ => {
            eprintln!("用法: happy-installer <install|uninstall>");
            std::process::exit(1);
        }
    };
    if let Err(e) = result {
        eprintln!("操作失败: {e}");
        std::process::exit(1);
    }
}

fn install() -> Result<(), String> {
    let dll = locate_dll()?;
    let dir = expand(INSTALL_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let target = std::path::Path::new(&dir).join(DLL_NAME);
    std::fs::copy(&dll, &target).map_err(|e| format!("拷贝 DLL 失败: {e}"))?;
    println!("已拷贝 {}", target.display());

    // regsvr32 静默注册（我们的 DllRegisterServer 内部自带 HKLM→HKCU 回退）
    let status = Command::new("regsvr32")
        .args(["/s", &target.to_string_lossy()])
        .status()
        .map_err(|e| format!("启动 regsvr32 失败: {e}"))?;
    if status.success() {
        println!("注册成功。请使用 Win+Space 切换到「Happy 输入法」。");
    } else {
        return Err("regsvr32 注册失败（请以管理员身份重试）".into());
    }
    Ok(())
}

fn uninstall() -> Result<(), String> {
    let dir = expand(INSTALL_DIR);
    let target = std::path::Path::new(&dir).join(DLL_NAME);
    if target.exists() {
        let status = Command::new("regsvr32")
            .args(["/s", "/u", &target.to_string_lossy()])
            .status()
            .map_err(|e| format!("启动 regsvr32 失败: {e}"))?;
        if !status.success() {
            eprintln!("警告: 反注册返回失败，继续删除文件");
        }
        std::fs::remove_file(&target).map_err(|e| format!("删除 DLL 失败: {e}"))?;
        println!("已删除 {}", target.display());
    } else {
        println!("未找到 {}", target.display());
    }
    Ok(())
}

/// 在 workspace 的 target 目录中定位编译产物 happy_ime.dll
fn locate_dll() -> Result<std::path::PathBuf, String> {
    let mut candidates = Vec::new();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    // 安装器通常位于 target/{debug,release}/，DLL 在同级
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(DLL_NAME));
        // 兼容 workspace 下 happy-installer 与 happy-ime 共用 target 根
        if let Some(root) = dir.parent() {
            candidates.push(root.join(DLL_NAME));
        }
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Err(format!(
        "未找到 {}，请先 cargo build --release（查找路径: {:?}）",
        DLL_NAME, candidates
    ))
}

fn expand(s: &str) -> String {
    // 简易环境变量展开（%ProgramFiles% 等）
    let mut out = s.to_string();
    for (k, v) in std::env::vars() {
        let pat = format!("%{k}%");
        if out.contains(&pat) {
            out = out.replace(&pat, &v);
        }
    }
    out
}
