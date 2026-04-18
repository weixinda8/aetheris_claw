use crate::api::ApiClient;
use crate::models::{SystemMetrics, TaskMetrics};
use dioxus::prelude::*;

#[component]
pub fn MetricsDashboard() -> Element {
    let system_metrics = use_signal(|| Option::<SystemMetrics>::None);
    let task_metrics = use_signal(|| Option::<TaskMetrics>::None);
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_metrics = move || {
        spawn({
            let mut system_metrics = system_metrics.clone();
            let mut task_metrics = task_metrics.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            async move {
                is_loading.set(true);
                error.set(None);

                let system_result = ApiClient::get_system_metrics().await;
                let task_result = ApiClient::get_task_metrics().await;

                match (system_result, task_result) {
                    (Ok(sys), Ok(task)) => {
                        system_metrics.set(Some(sys));
                        task_metrics.set(Some(task));
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        error.set(Some(e));
                    }
                }

                is_loading.set(false);
            }
        });
    };

    use_effect(move || {
        load_metrics();
    });

    rsx! {
        div { class: "max-w-7xl mx-auto",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "系统指标" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_metrics(),
                    "刷新"
                }
            }

            if *is_loading.read() {
                div { class: "text-center py-12",
                    p { class: "text-gray-600", "加载中..." }
                }
            } else if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                    p { "加载指标失败: {err}" }
                    button {
                        class: "mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md",
                        onclick: move |_| load_metrics(),
                        "重试"
                    }
                }
            } else {
                div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                    {
                        let sys = system_metrics.read();
                        if let Some(s) = sys.as_ref() {
                            rsx! { SystemMetricsCard { metrics: s.clone() } }
                        } else {
                            rsx! {}
                        }
                    }
                    {
                        let task = task_metrics.read();
                        if let Some(t) = task.as_ref() {
                            rsx! { TaskMetricsCard { metrics: t.clone() } }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SystemMetricsCard(metrics: SystemMetrics) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-6", "系统资源" }
            div { class: "grid grid-cols-2 gap-4",
                MetricItem {
                    label: "CPU 使用率",
                    value: format!("{:.1}%", metrics.cpu_usage_percent),
                    color: "#3b82f6",
                    icon: "💻"
                }
                MetricItem {
                    label: "内存使用",
                    value: format!("{:.1} GB", metrics.memory_used_gb),
                    color: "#8b5cf6",
                    icon: "🧠"
                }
                MetricItem {
                    label: "内存总量",
                    value: format!("{:.1} GB", metrics.memory_total_gb),
                    color: "#6366f1",
                    icon: "📊"
                }
                MetricItem {
                    label: "活跃连接",
                    value: metrics.active_connections.to_string(),
                    color: "#10b981",
                    icon: "🔗"
                }
                MetricItem {
                    label: "运行时间",
                    value: format_duration(metrics.uptime_seconds),
                    color: "#f59e0b",
                    icon: "⏱️"
                }
                MetricItem {
                    label: "Agent 数量",
                    value: metrics.agent_count.to_string(),
                    color: "#ec4899",
                    icon: "🤖"
                }
            }
        }
    }
}

#[component]
fn TaskMetricsCard(metrics: TaskMetrics) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-6", "任务统计" }
            div { class: "grid grid-cols-2 gap-4",
                MetricItem {
                    label: "总任务数",
                    value: metrics.total_tasks.to_string(),
                    color: "#3b82f6",
                    icon: "📋"
                }
                MetricItem {
                    label: "运行中",
                    value: metrics.running_tasks.to_string(),
                    color: "#10b981",
                    icon: "▶️"
                }
                MetricItem {
                    label: "已完成",
                    value: metrics.completed_tasks.to_string(),
                    color: "#059669",
                    icon: "✅"
                }
                MetricItem {
                    label: "失败",
                    value: metrics.failed_tasks.to_string(),
                    color: "#dc2626",
                    icon: "❌"
                }
                MetricItem {
                    label: "成功率",
                    value: format!("{:.1}%", metrics.success_rate_percent),
                    color: "#7c3aed",
                    icon: "📈"
                }
                MetricItem {
                    label: "平均耗时",
                    value: format_duration(metrics.avg_duration_ms as i64),
                    color: "#f59e0b",
                    icon: "⏰"
                }
                MetricItem {
                    label: "总 Token",
                    value: metrics.total_tokens.to_string(),
                    color: "#0891b2",
                    icon: "🔤"
                }
                MetricItem {
                    label: "总成本",
                    value: format!("${:.4}", metrics.total_cost_usd),
                    color: "#d97706",
                    icon: "💰"
                }
            }
        }
    }
}

#[component]
fn MetricItem(label: &'static str, value: String, color: &'static str, icon: &'static str) -> Element {
    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4 border border-gray-200",
            div { class: "flex items-center justify-between mb-2",
                span { class: "text-gray-500 text-sm", "{label}" }
                span { class: "text-2xl", "{icon}" }
            }
            p {
                class: "text-2xl font-bold",
                style: "color: {color};",
                "{value}"
            }
        }
    }
}

fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{} 秒", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{} 分 {} 秒", minutes, secs)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{} 小时 {} 分", hours, minutes)
    }
}
