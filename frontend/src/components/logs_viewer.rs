use crate::api::ApiClient;
use crate::models::{AuditLog, Alert};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogsViewTab {
    AuditLogs,
    Alerts,
}

#[component]
pub fn LogsViewer() -> Element {
    let mut active_tab = use_signal(|| LogsViewTab::AuditLogs);
    let audit_logs = use_signal(|| Vec::<AuditLog>::new());
    let alerts = use_signal(|| Vec::<Alert>::new());
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_data = move || {
        spawn({
            let mut audit_logs = audit_logs.clone();
            let mut alerts = alerts.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            async move {
                is_loading.set(true);
                error.set(None);

                let audit_result = ApiClient::get_audit_logs(None, None, None, None).await;
                let alerts_result = ApiClient::get_alerts(None, None).await;

                match (audit_result, alerts_result) {
                    (Ok(logs), Ok(al)) => {
                        audit_logs.set(logs);
                        alerts.set(al);
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
        load_data();
    });

    rsx! {
        div { class: "max-w-7xl mx-auto",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "日志与告警" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_data(),
                    "刷新"
                }
            }

            div { class: "bg-white rounded-lg shadow-md p-6",
                div { class: "flex space-x-2 mb-6",
                    button {
                        class: if *active_tab.read() == LogsViewTab::AuditLogs {
                            "px-4 py-2 bg-blue-600 text-white rounded-md"
                        } else {
                            "px-4 py-2 bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300"
                        },
                        onclick: move |_| active_tab.set(LogsViewTab::AuditLogs),
                        "审计日志"
                    }
                    button {
                        class: if *active_tab.read() == LogsViewTab::Alerts {
                            "px-4 py-2 bg-blue-600 text-white rounded-md"
                        } else {
                            "px-4 py-2 bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300"
                        },
                        onclick: move |_| active_tab.set(LogsViewTab::Alerts),
                        "系统告警"
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
                            onclick: move |_| load_data(),
                            "重试"
                        }
                    }
                } else {
                    match *active_tab.read() {
                        LogsViewTab::AuditLogs => rsx! { AuditLogsList { logs: audit_logs.read().clone() } },
                        LogsViewTab::Alerts => rsx! { AlertsList { alerts: alerts.read().clone() } },
                    }
                }
            }
        }
    }
}

#[component]
fn AuditLogsList(logs: Vec<AuditLog>) -> Element {
    if logs.is_empty() {
        rsx! {
            div { class: "text-center py-12",
                p { class: "text-gray-600", "暂无审计日志" }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                for log in logs.iter() {
                    AuditLogItem { log: log.clone() }
                }
            }
        }
    }
}

#[component]
fn AuditLogItem(log: AuditLog) -> Element {
    let level_class = match log.level.as_str() {
        "info" => "bg-blue-100 text-blue-800",
        "warn" => "bg-yellow-100 text-yellow-800",
        "error" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    };
    let timestamp_str = log.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
    let task_id_text = log.task_id.map(|id| format!("任务ID: {}", id));
    let agent_id_text = log.agent_id.map(|id| format!("Agent: {}", id));

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4 border border-gray-200",
            div { class: "flex justify-between items-start mb-2",
                div { class: "flex items-center gap-3",
                    span {
                        class: "px-2 py-1 rounded-full text-xs font-medium {level_class}",
                        "{log.level.to_uppercase()}"
                    }
                    span { class: "text-sm font-medium text-gray-900", "{log.event_type}" }
                }
                span { class: "text-xs text-gray-500", "{timestamp_str}" }
            }
            p { class: "text-sm text-gray-700", "{log.message}" }
            if let Some(text) = task_id_text {
                p { class: "text-xs text-gray-500 mt-1", "{text}" }
            }
            if let Some(text) = agent_id_text {
                p { class: "text-xs text-gray-500", "{text}" }
            }
        }
    }
}

#[component]
fn AlertsList(alerts: Vec<Alert>) -> Element {
    if alerts.is_empty() {
        rsx! {
            div { class: "text-center py-12",
                p { class: "text-gray-600", "暂无系统告警" }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                for alert in alerts.iter() {
                    AlertItem { alert: alert.clone() }
                }
            }
        }
    }
}

#[component]
fn AlertItem(alert: Alert) -> Element {
    let (severity_class, severity_text) = match alert.severity.as_str() {
        "critical" => ("bg-red-100 text-red-800", "严重"),
        "warning" => ("bg-yellow-100 text-yellow-800", "警告"),
        "info" => ("bg-blue-100 text-blue-800", "信息"),
        _ => ("bg-gray-100 text-gray-800", "未知"),
    };
    let timestamp_str = alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
    let task_id_text = alert.task_id.map(|id| format!("任务ID: {}", id));

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4 border border-gray-200",
            div { class: "flex justify-between items-start mb-2",
                div { class: "flex items-center gap-3",
                    span {
                        class: "px-2 py-1 rounded-full text-xs font-medium {severity_class}",
                        "{severity_text}"
                    }
                    span { class: "text-sm font-medium text-gray-900", "{alert.alert_type}" }
                }
                span { class: "text-xs text-gray-500", "{timestamp_str}" }
            }
            p { class: "text-sm text-gray-700", "{alert.message}" }
            if let Some(text) = task_id_text {
                p { class: "text-xs text-gray-500 mt-1", "{text}" }
            }
        }
    }
}
