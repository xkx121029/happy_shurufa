//! 编程命令/片段联想。
//!
//! 内置词库为 `命令\t说明` 的 TSV；匹配当前输入 token 的前缀，
//! 按用户频率（FreqMap）优先、其次命令长度排序。

use crate::freq::FreqMap;

/// 内置命令词库（命令 / 中文说明）。
pub const COMMANDS: &[(&str, &str)] = &[
    // git
    ("git", "版本控制"),
    ("git status", "查看工作区状态"),
    ("git log", "查看提交历史"),
    ("git diff", "查看改动"),
    ("git add", "暂存改动"),
    ("git commit -m \"\"", "提交"),
    ("git push", "推送到远程"),
    ("git pull", "拉取远程"),
    ("git fetch", "获取远程更新"),
    ("git branch", "分支列表"),
    ("git checkout", "切换分支"),
    ("git switch", "切换分支"),
    ("git merge", "合并分支"),
    ("git rebase", "变基"),
    ("git stash", "暂存工作区"),
    ("git stash pop", "恢复暂存"),
    ("git clone", "克隆仓库"),
    ("git remote -v", "查看远程"),
    ("git cherry-pick", "拣选提交"),
    ("git revert", "回滚提交"),
    ("git reset", "重置"),
    ("git blame", "逐行追溯"),
    ("git tag", "标签"),
    ("git show", "查看提交内容"),
    ("git restore", "还原文件"),
    ("git clean -fd", "清理未跟踪文件"),
    // npm / pnpm / yarn
    ("npm init", "初始化项目"),
    ("npm install", "安装依赖"),
    ("npm run dev", "启动开发服务"),
    ("npm run build", "构建"),
    ("npm run test", "测试"),
    ("npm run lint", "代码检查"),
    ("npm start", "启动"),
    ("npm uninstall", "卸载依赖"),
    ("npm update", "更新依赖"),
    ("npx", "执行包命令"),
    ("pnpm install", "安装依赖"),
    ("pnpm add", "添加依赖"),
    ("pnpm remove", "移除依赖"),
    ("pnpm dev", "启动开发服务"),
    ("pnpm build", "构建"),
    ("yarn add", "添加依赖"),
    ("yarn install", "安装依赖"),
    ("yarn dev", "启动开发服务"),
    // cargo / rust
    ("cargo new", "新建项目"),
    ("cargo init", "初始化项目"),
    ("cargo build", "构建"),
    ("cargo build --release", "发布构建"),
    ("cargo run", "运行"),
    ("cargo test", "测试"),
    ("cargo check", "快速检查"),
    ("cargo clippy", "静态检查"),
    ("cargo fmt", "格式化"),
    ("cargo add", "添加依赖"),
    ("cargo doc", "生成文档"),
    ("cargo publish", "发布 crate"),
    ("rustup update", "更新工具链"),
    // docker / k8s
    ("docker ps", "容器列表"),
    ("docker images", "镜像列表"),
    ("docker run", "运行容器"),
    ("docker exec -it", "进入容器"),
    ("docker build -t", "构建镜像"),
    ("docker compose up -d", "启动编排服务"),
    ("docker compose down", "停止编排服务"),
    ("docker logs -f", "跟踪日志"),
    ("docker stop", "停止容器"),
    ("docker rm", "删除容器"),
    ("docker rmi", "删除镜像"),
    ("docker pull", "拉取镜像"),
    ("docker system prune", "清理资源"),
    ("kubectl get pods", "Pod 列表"),
    ("kubectl get svc", "服务列表"),
    ("kubectl describe", "资源详情"),
    ("kubectl logs", "容器日志"),
    ("kubectl apply -f", "应用清单"),
    ("kubectl delete", "删除资源"),
    ("kubectl exec -it", "进入容器"),
    ("kubectl rollout restart", "滚动重启"),
    // PowerShell / cmd
    ("Get-ChildItem", "列出目录内容"),
    ("Get-Content", "读取文件"),
    ("Set-Location", "切换目录"),
    ("Get-Process", "进程列表"),
    ("Get-Service", "服务列表"),
    ("Get-Command", "查找命令"),
    ("Get-Help", "帮助"),
    ("Select-String", "文本搜索"),
    ("Copy-Item", "复制"),
    ("Move-Item", "移动"),
    ("Remove-Item", "删除"),
    ("New-Item", "新建"),
    ("Test-Path", "路径检测"),
    ("Get-Date", "当前时间"),
    ("Measure-Object", "统计对象"),
    ("Invoke-WebRequest", "HTTP 请求"),
    ("ipconfig", "网络配置"),
    ("ping", "网络连通"),
    ("netstat -ano", "端口占用"),
    ("tasklist", "进程列表"),
    ("taskkill", "结束进程"),
    ("cls", "清屏"),
    ("echo", "输出"),
    ("mkdir", "新建目录"),
    ("rd", "删除目录"),
    ("dir", "列出目录"),
    ("cd", "切换目录"),
    ("where", "查找程序"),
    ("scoop", "Windows 包管理"),
    ("winget", "Windows 包管理"),
    ("choco", "Windows 包管理"),
    // python / node / java
    ("python -m venv", "创建虚拟环境"),
    ("pip install", "安装包"),
    ("pip uninstall", "卸载包"),
    ("pip list", "包列表"),
    ("python main.py", "运行脚本"),
    ("uv venv", "创建虚拟环境"),
    ("uv sync", "同步依赖"),
    ("uv run", "运行"),
    ("node -v", "Node 版本"),
    ("java -version", "Java 版本"),
    ("mvn clean package", "Maven 打包"),
    ("gradle build", "Gradle 构建"),
    ("dotnet new", "新建项目"),
    ("dotnet build", "构建"),
    ("dotnet run", "运行"),
    ("dotnet test", "测试"),
    ("dotnet publish", "发布"),
    ("dotnet add package", "添加 NuGet 包"),
    ("flutter create", "新建 Flutter 项目"),
    ("flutter run", "运行 Flutter"),
    ("flutter build apk", "构建 APK"),
    ("adb devices", "设备列表"),
    ("adb install", "安装应用"),
    ("adb logcat", "查看日志"),
    // 常用片段
    ("println!", "Rust 打印"),
    ("console.log()", "JS 打印"),
    ("System.out.println()", "Java 打印"),
    ("Console.WriteLine()", "C# 打印"),
    ("print()", "Python 打印"),
    ("fmt.Println()", "Go 打印"),
    ("function", "JS 函数"),
    ("interface", "接口"),
    ("return", "返回"),
    ("public static void main", "Java 主函数"),
    ("def main", "Python 主函数"),
    ("if __name__ == \"__main__\"", "Python 入口"),
];

/// 一条联想结果：文本 + 说明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandHit {
    pub text: String,
    pub desc: String,
    /// 命令首词与输入 token 的共同前缀长度（用于回退删除后补全）。
    pub first_word_prefix: usize,
}

/// 根据当前 token 联想命令。
///
/// `token` 为光标前最后一个由字母数字组成的串；返回最多 `limit` 条结果。
pub fn suggest(token: &str, freq: &FreqMap, limit: usize) -> Vec<CommandHit> {
    let token = token.trim();
    if token.is_empty() || limit == 0 {
        return Vec::new();
    }
    let token_lower = token.to_lowercase();
    let mut hits: Vec<CommandHit> = Vec::new();
    for (cmd, desc) in COMMANDS {
        let Some(first_word) = cmd.split_whitespace().next() else {
            continue;
        };
        if !first_word.to_lowercase().starts_with(&token_lower) {
            continue;
        }
        hits.push(CommandHit {
            text: cmd.to_string(),
            desc: desc.to_string(),
            first_word_prefix: token.chars().count(),
        });
    }
    if hits.is_empty() {
        return hits;
    }
    hits.sort_by(|a, b| {
        let fa = freq.get(&a.text);
        let fb = freq.get(&b.text);
        fb.cmp(&fa)
            .then_with(|| a.text.len().cmp(&b.text.len()))
            .then_with(|| a.text.cmp(&b.text))
    });
    hits.truncate(limit);
    hits
}

/// 提交命中时需要插入的剩余部分：
/// 已输入 token 是命令首词的前缀，补全首词剩余 + 首词之后的全部内容。
pub fn remainder(hit: &CommandHit) -> &str {
    let chars: Vec<char> = hit.text.chars().collect();
    let n = hit.first_word_prefix.min(chars.len());
    &hit.text[n..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 前缀匹配() {
        let freq = FreqMap::default();
        let hits = suggest("git", &freq, 9);
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|h| h.text == "git status"));
    }

    #[test]
    fn 频率优先() {
        let mut freq = FreqMap::default();
        freq.bump("git push");
        freq.bump("git push");
        let hits = suggest("git", &freq, 9);
        assert_eq!(hits[0].text, "git push");
    }

    #[test]
    fn 命令未命中为空() {
        let freq = FreqMap::default();
        assert!(suggest("zzzqqq", &freq, 9).is_empty());
    }

    #[test]
    fn 补全剩余() {
        let freq = FreqMap::default();
        let hits = suggest("che", &freq, 9);
        let hit = hits.iter().find(|h| h.text == "git checkout").unwrap();
        assert_eq!(remainder(hit), "ckout");
    }

    #[test]
    fn token_来自首词() {
        let freq = FreqMap::default();
        let hits = suggest("dock", &freq, 9);
        assert!(hits.iter().any(|h| h.text == "docker ps"));
    }
}
