use regex::Regex;

#[derive(Clone, Debug)]
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

    /// 将工具调用传入的 ignore 正则与启动时的 --ignore 合并。
    /// `extra` 为空或空白时，仅使用启动配置。
    pub fn with_call_ignore(&self, extra: Option<&str>) -> Result<Self, String> {
        let mut ignore_patterns = self.ignore_patterns.clone();
        if let Some(pattern) = extra.map(str::trim).filter(|s| !s.is_empty()) {
            let re = Regex::new(pattern).map_err(|e| format!("无效的 ignore 正则: {}", e))?;
            ignore_patterns.push(re);
        }
        Ok(Config { ignore_patterns })
    }

    /// 判断给定路径（文件名或目录名片段）是否应该被忽略
    pub fn should_ignore(&self, path_str: &str) -> bool {
        self.ignore_patterns.iter().any(|re| re.is_match(path_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_call_ignore_merges_startup_and_call_patterns() {
        let base = Config {
            ignore_patterns: vec![Regex::new("node_modules").unwrap()],
        };
        let merged = base.with_call_ignore(Some("target|dist")).unwrap();
        assert!(merged.should_ignore("node_modules"));
        assert!(merged.should_ignore("target"));
        assert!(merged.should_ignore("dist"));
        assert!(!merged.should_ignore("src"));
    }

    #[test]
    fn with_call_ignore_ignores_blank_extra() {
        let base = Config {
            ignore_patterns: vec![Regex::new("node_modules").unwrap()],
        };
        let merged = base.with_call_ignore(Some("   ")).unwrap();
        assert_eq!(merged.ignore_patterns.len(), 1);
        assert!(merged.should_ignore("node_modules"));
    }

    #[test]
    fn with_call_ignore_rejects_invalid_regex() {
        let base = Config {
            ignore_patterns: Vec::new(),
        };
        let err = base.with_call_ignore(Some("(")).unwrap_err();
        assert!(err.contains("无效的 ignore 正则"));
    }
}
