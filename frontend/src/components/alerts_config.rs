use crate::api::ApiClient;
use crate::models::Alert;
use dioxus::prelude::*;

#[component]
pub fn AlertsConfig() -> Element {
    let alerts = use_signal(|| Vec::<Alert>::new());
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_alerts = move || {
        spawn({
            let mut alerts = alerts.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            async move {
                is_loading.set(true);
                error.set(None);
                match ApiClient::get_alerts(None, None).await {
                    Ok(fetched_alerts) => {
                        alerts.set(fetched_alerts);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                is_loading.set(false);
            }
        });
    };

    use_effect(move || {
        load_alerts();
    });

    rsx! {
        div { class: "max-w-7xl mx-auto",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "告警与自愈配置" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_alerts(),
                    "刷新"
                }
            }

            if *is_loading.read() {
                div { class: "text-center py-12",
                    p { class: "text-gray-600", "加载中..." }
                }
            } else if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                    p { "加载失败: {err}" }
                    button {
                        class: "mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md",
                        onclick: move |_| load_alerts(),
                        "重试"
                    }
                }
            } else {
                div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                    AlertsListCard { alerts: alerts.read().clone() }
                    SelfHealingConfigCard {}
                }
            }
        }
    }
}

#[component]
fn AlertsListCard(alerts: Vec<Alert>) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "最近告警" }
            if alerts.is_empty() {
                div { class: "text-center py-8",
                    p { class: "text-gray-600", "暂无告警" }
                }
            } else {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    for alert in alerts.iter().take(10) {
                        AlertListItem { alert: alert.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn AlertListItem(alert: Alert) -> Element {
    let (severity_class, severity_text) = match alert.severity.as_str() {
        "critical" => ("bg-red-100 text-red-800", "严重"),
        "warning" => ("bg-yellow-100 text-yellow-800", "警告"),
        "info" => ("bg-blue-100 text-blue-800", "信息"),
        _ => ("bg-gray-100 text-gray-800", "未知"),
    };
    let timestamp_str = alert.timestamp.format("%H:%M:%S").to_string();

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3 border border-gray-200",
            div { class: "flex justify-between items-start mb-2",
                div { class: "flex items-center gap-2",
                    span {
                        class: "px-2 py-1 rounded-full text-xs font-medium {severity_class}",
                        "{severity_text}"
                    }
                    span { class: "text-sm font-medium text-gray-900", "{alert.alert_type}" }
                }
                span { class: "text-xs text-gray-500", "{timestamp_str}" }
            }
            p { class: "text-sm text-gray-700", "{alert.message}" }
        }
    }
}

#[component]
fn SelfHealingConfigCard() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "自愈配置" }
            div { class: "space-y-4",
                div { class: "flex items-center justify-between",
                    label { class: "text-gray-700 font-medium", "自动故障恢复" }
                    span { class: "px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800", "已启用" }
                }

                div { class: "space-y-2",
                    label { class: "text-gray-700 font-medium block", "告警阈值（连续失败次数）" }
                    span { class: "text-xl font-bold text-gray-800", "3" }
                }

                div { class: "pt-4 border-t border-gray-200",
                    h4 { class: "font-medium text-gray-800 mb-3", "自愈策略" }
                    div { class: "space-y-2",
                        StrategyItem { name: "自动重试", enabled: true }
                        StrategyItem { name: "回滚到上一版本", enabled: true }
                        StrategyItem { name: "切换备用 Agent", enabled: true }
                        StrategyItem { name: "人工介入通知", enabled: true }
                    }
                }
            }
        }
    }
}

#[component]
fn StrategyItem(name: &'static str, enabled: bool) -> Element {
    rsx! {
        div { class: "flex items-center justify-between bg-gray-50 rounded-lg p-3",
            span { class: "text-gray-700", "{name}" }
            span {
                class: if enabled {
                    "px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800"
                } else {
                    "px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-800"
                },
                if enabled { "已启用" } else { "已禁用" }
            }
        }
    }
}
