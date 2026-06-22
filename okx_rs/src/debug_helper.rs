use crate::error::Error;
use serde_json::Value;

/// 调试助手，用于分析API错误和响应
pub struct DebugHelper;

impl DebugHelper {
    /// 分析JSON错误并提供诊断建议
    pub fn analyze_json_error(error: &serde_json::Error, response_body: &str) {
        println!("🔍 JSON错误详细分析:");
        println!("   错误类型: {}", error);
        println!("   出错位置: 第{}行, 第{}列", error.line(), error.column());

        if let Some(context) =
            Self::extract_error_context(response_body, error.line(), error.column())
        {
            println!("   出错内容: {}", context);
        }

        // 分析常见问题
        if response_body.contains("\"data\":[{}]") {
            println!("⚠️  检测到空数据对象，这可能是API返回了空结果");
        }

        if response_body.contains("\"code\":\"1\"") {
            println!("⚠️  检测到API错误代码1，请检查API参数");
        }

        if response_body.len() < 50 {
            println!("⚠️  响应内容过短，可能是网络问题或API服务异常");
        }

        if !response_body.starts_with('{') && !response_body.starts_with('[') {
            println!("⚠️  响应不是有效的JSON格式，可能是HTML错误页面或其他格式");
        }
    }

    /// 提取错误上下文
    fn extract_error_context(response: &str, line: usize, column: usize) -> Option<String> {
        let lines: Vec<&str> = response.lines().collect();
        if line > 0 && line <= lines.len() {
            let error_line = lines[line - 1];
            let start = if column > 20 { column - 20 } else { 0 };
            let end = if column + 20 < error_line.len() {
                column + 20
            } else {
                error_line.len()
            };
            Some(format!("...{}...", &error_line[start..end]))
        } else {
            None
        }
    }

    /// 美化JSON响应用于调试
    pub fn pretty_print_json(json_str: &str) -> String {
        match serde_json::from_str::<Value>(json_str) {
            Ok(value) => {
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| json_str.to_string())
            }
            Err(_) => json_str.to_string(),
        }
    }

    /// 诊断OKX API错误
    pub fn diagnose_okx_error(error: &Error) {
        match error {
            Error::JsonError(_json_err) => {
                println!("🔍 JSON解析错误诊断:");
                println!("   这通常表示OKX API的响应格式与期望不符");
                println!("   建议: 检查API文档是否有更新，或联系OKX技术支持");
            }
            Error::OkxApiError {
                code,
                message,
                smg: _,
            } => {
                println!("🔍 OKX API错误诊断:");
                println!("   错误代码: {}", code);
                println!("   错误信息: {}", message);
                Self::provide_error_code_suggestion(code);
            }
            Error::HttpError(http_err) => {
                println!("🔍 网络错误诊断:");
                println!("   错误详情: {}", http_err);
                println!("   建议: 检查网络连接和防火墙设置");
            }
            _ => {
                println!("🔍 其他错误: {:?}", error);
            }
        }
    }

    /// 根据错误代码提供建议
    fn provide_error_code_suggestion(code: &str) {
        match code {
            "50011" => println!("   建议: 请求头无效，检查API Key和签名"),
            "50013" => println!("   建议: 无效的签名，检查API Secret"),
            "50014" => println!("   建议: 时间戳过期，检查系统时间"),
            "51000" => println!("   建议: 参数错误，检查请求参数"),
            "51008" => println!("   建议: 订单不存在"),
            "51020" => println!("   建议: 余额不足"),
            "51094" => println!("   建议: 不支持的订单类型或参数"),
            _ => println!("   建议: 查阅OKX API文档获取详细信息"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_print_json() {
        let json_str = r#"{"code":"0","data":[{"test":"value"}]}"#;
        let pretty = DebugHelper::pretty_print_json(json_str);
        assert!(pretty.contains("\"code\": \"0\""));
    }
}
