use regex::Regex;

pub struct Config {
    pub ignore_patterns: Vec<Regex>,
}

impl Config {
    /// 从命令行参数中解析 --ignore 选项，构建全局配置
    /// 用法：file-search-mcp --ignore node_modules --ignore \.git --ignore target
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut ignore_patterns = Vec::new();

        let mut i = 1;
        while i < args.len() {
            if args[i] == "--ignore" {
                if let Some(pattern) = args.get(i + 1) {
                    match Regex::new(pattern) {
                        Ok(re) => ignore_patterns.push(re),
                        Err(e) => eprintln!("[WARN] 忽略无效的正则模式 {:?}: {}", pattern, e),
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        Config { ignore_patterns }
    }

    /// 判断给定路径（文件名或目录名片段）是否应该被忽略
    pub fn should_ignore(&self, path_str: &str) -> bool {
        self.ignore_patterns.iter().any(|re| re.is_match(path_str))
    }
}
