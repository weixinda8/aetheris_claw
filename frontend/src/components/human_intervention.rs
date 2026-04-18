use crate::api::ApiClient;
use crate::models::{Task, TaskStatus};
use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InterventionAction {
    Pause,
    Resume,
    Cancel,
    Approve,
    Reject,
}

#[component]
pub fn HumanIntervention() -> Element {
    let pending_tasks = use_signal(|| Vec::<Task>::new());
    let all_tasks = use_signal(|| Vec::<Task>::new());
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_tasks = move || {
        spawn({
            let mut pending_tasks = pending_tasks.clone();
            let mut all_tasks = all_tasks.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            async move {
                is_loading.set(true);
                error.set(None);
                match ApiClient::get_tasks().await {
                    Ok(tasks) => {
                        let pending = tasks.iter()
                            .filter(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::Queued | TaskStatus::Running))
                            .cloned()
                            .collect::<Vec<_>>();
                        pending_tasks.set(pending);
                        all_tasks.set(tasks);
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
        load_tasks();
    });

    rsx! {
        div { class: "max-w-7xl mx-auto",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "人工干预控制台" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_tasks(),
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
                        onclick: move |_| load_tasks(),
                        "重试"
                    }
                }
            } else {
                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                    PendingTasksCard {
                        tasks: pending_tasks.read().clone()
                    }
                    AllTasksCard { tasks: all_tasks.read().clone() }
                    InterventionHistoryCard {}
                }
            }
        }
    }
}

#[component]
fn PendingTasksCard(
    tasks: Vec<Task>,
) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "待处理任务" }
            if tasks.is_empty() {
                div { class: "text-center py-8",
                    p { class: "text-gray-600", "暂无待处理任务" }
                }
            } else {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    for task in tasks.iter() {
                        PendingTaskItem {
                            task: task.clone()
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PendingTaskItem(
    task: Task,
) -> Element {
    let (status_class, status_text) = get_status_info(&task.status);

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4 border border-gray-200",
            div { class: "flex justify-between items-start mb-3",
                div {
                    h4 { class: "font-medium text-gray-900", "{task.title}" }
                    p { class: "text-sm text-gray-600 mt-1", "{task.description}" }
                }
                span {
                    class: "px-2 py-1 rounded-full text-xs font-medium {status_class}",
                    "{status_text}"
                }
            }
            div { class: "flex gap-2",
                if matches!(task.status, TaskStatus::Running) {
                    button {
                        class: "px-3 py-1 bg-yellow-100 text-yellow-700 hover:bg-yellow-200 rounded-md text-sm transition-colors",
                        "暂停"
                    }
                }
                if matches!(task.status, TaskStatus::Pending | TaskStatus::Queued) {
                    button {
                        class: "px-3 py-1 bg-green-100 text-green-700 hover:bg-green-200 rounded-md text-sm transition-colors",
                        "开始"
                    }
                }
                button {
                    class: "px-3 py-1 bg-red-100 text-red-700 hover:bg-red-200 rounded-md text-sm transition-colors",
                    "取消"
                }
            }
        }
    }
}

#[component]
fn AllTasksCard(tasks: Vec<Task>) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "所有任务" }
            if tasks.is_empty() {
                div { class: "text-center py-8",
                    p { class: "text-gray-600", "暂无任务" }
                }
            } else {
                div { class: "space-y-2 max-h-96 overflow-y-auto",
                    for task in tasks.iter().take(10) {
                        TaskSummaryItem { task: task.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskSummaryItem(task: Task) -> Element {
    let (status_class, status_text) = get_status_info(&task.status);
    let created_at_str = task.created_at.format("%Y-%m-%d %H:%M").to_string();

    rsx! {
        div { class: "flex justify-between items-center p-3 bg-gray-50 rounded-lg",
            div {
                p { class: "text-sm font-medium text-gray-900", "{task.title}" }
                p { class: "text-xs text-gray-500", "{created_at_str}" }
            }
            span {
                class: "px-2 py-1 rounded-full text-xs font-medium {status_class}",
                "{status_text}"
            }
        }
    }
}

#[component]
fn InterventionHistoryCard() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "干预历史" }
            div { class: "space-y-3",
                HistoryItem {
                    action: "暂停任务",
                    task_title: "优化构建流程",
                    time: "2024-01-15 14:30:00",
                    user: "管理员"
                }
                HistoryItem {
                    action: "恢复任务",
                    task_title: "优化构建流程",
                    time: "2024-01-15 14:35:00",
                    user: "管理员"
                }
                HistoryItem {
                    action: "批准执行",
                    task_title: "生成报告",
                    time: "2024-01-15 10:00:00",
                    user: "审批者"
                }
            }
        }
    }
}

#[component]
fn HistoryItem(action: &'static str, task_title: &'static str, time: &'static str, user: &'static str) -> Element {
    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3 border border-gray-200",
            div { class: "flex justify-between items-start mb-1",
                span { class: "text-sm font-medium text-gray-900", "{action}" }
                span { class: "text-xs text-gray-500", "{time}" }
            }
            p { class: "text-sm text-gray-600", "{task_title}" }
            p { class: "text-xs text-gray-500", "操作者: {user}" }
        }
    }
}

fn get_status_info(status: &TaskStatus) -> (&'static str, &'static str) {
    match status {
        TaskStatus::Pending => ("bg-yellow-100 text-yellow-800", "待处理"),
        TaskStatus::Queued => ("bg-blue-100 text-blue-800", "已排队"),
        TaskStatus::Running => ("bg-green-100 text-green-800", "运行中"),
        TaskStatus::Completed => ("bg-purple-100 text-purple-800", "已完成"),
        TaskStatus::Failed => ("bg-red-100 text-red-800", "失败"),
        TaskStatus::Paused => ("bg-gray-100 text-gray-800", "已暂停"),
        TaskStatus::Cancelled => ("bg-gray-100 text-gray-800", "已取消"),
    }
}
