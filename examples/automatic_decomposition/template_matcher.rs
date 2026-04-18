use std::collections::HashMap;

pub struct TaskTemplateMatcher {
    keyword_to_template: HashMap<String, String>,
}

impl TaskTemplateMatcher {
    pub fn new() -> Self {
        let mut keyword_to_template = HashMap::new();

        let chemical_keywords = vec![
            "化工生产订单",
            "生产订单",
            "化工订单",
            "生产报告",
            "最终生产报告",
            "化工生产",
            "订单处理",
        ];

        for keyword in chemical_keywords {
            keyword_to_template.insert(
                keyword.to_string(),
                "chemical-production-order".to_string(),
            );
        }

        Self { keyword_to_template }
    }

    pub fn match_template(&self, task_description: &str) -> Option<String> {
        let lower_desc = task_description.to_lowercase();

        for (keyword, template_id) in &self.keyword_to_template {
            if lower_desc.contains(&keyword.to_lowercase()) {
                return Some(template_id.clone());
            }
        }

        None
    }
}

impl Default for TaskTemplateMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_chemical_production_order() {
        let matcher = TaskTemplateMatcher::new();
        
        let test_cases = vec![
            "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
            "化工生产订单处理",
            "生产订单需要处理",
            "化工订单来了",
            "需要生成生产报告",
            "最终生产报告生成",
        ];

        for test_case in test_cases {
            let result = matcher.match_template(test_case);
            assert_eq!(result, Some("chemical-production-order".to_string()));
        }
    }

    #[test]
    fn test_no_match() {
        let matcher = TaskTemplateMatcher::new();
        
        let test_cases = vec![
            "今天天气真好",
            "我想吃饭",
            "写一段代码",
            "分析一下数据",
        ];

        for test_case in test_cases {
            let result = matcher.match_template(test_case);
            assert_eq!(result, None);
        }
    }
}
