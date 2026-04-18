use crate::api::ApiClient;
use crate::models::{Task, TaskStatus};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn ProgressVisualization(task_id: Uuid) -> Element {
    let task = use_signal(|| Option::<Task>::None);
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_task = move || {
        spawn({
            let mut task = task.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            let task_id = task_id;
            async move {
                is_loading.set(true);
                error.set(None);
                match ApiClient::get_task(task_id).await {
                    Ok(fetched_task) => {
                        task.set(Some(fetched_task));
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
        load_task();
    });

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "任务执行进度" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_task(),
                    "刷新"
                }
            }

            if *is_loading.read() {
                div { class: "text-center py-12",
                    p { class: "text-gray-600", "加载中..." }
                }
            } else if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                    p { "加载任务失败: {err}" }
                    button {
                        class: "mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md",
                        onclick: move |_| load_task(),
                        "重试"
                    }
                }
            } else if let Some(task_data) = task.read().as_ref() {
                ProgressView { task: task_data.clone() }
            } else {
                div { class: "text-center py-12 bg-white rounded-lg shadow-md",
                    p { class: "text-gray-600", "暂无任务数据" }
                }
            }
        }
    }
}

#[component]
fn ProgressView(task: Task) -> Element {
    let progress_percent = calculate_progress_percent(&task.status);
    let (status_color, status_text) = get_status_info(&task.status);

    rsx! {
        div { class: "space-y-6",
            div { class: "bg-white rounded-lg shadow-md p-6",
                div { class: "flex justify-between items-center mb-4",
                    h3 { class: "text-xl font-semibold text-gray-800", "{task.title}" }
                    span {
                        class: "px-3 py-1 rounded-full text-sm font-medium text-white",
                        style: "background-color: {status_color};",
                        "{status_text}"
                    }
                }

                div { class: "space-y-2",
                    div { class: "flex justify-between text-sm text-gray-600",
                        span { "进度" }
                        span { "{progress_percent}%" }
                    }
                    div { class: "w-full bg-gray-200 rounded-full h-4 overflow-hidden",
                        div {
                            class: "h-full rounded-full transition-all duration-500",
                            style: "width: {progress_percent}%; background-color: {status_color};",
                        }
                    }
                }

                div { class: "mt-6 pt-4 border-t border-gray-200",
                    p { class: "text-sm text-gray-700", "{task.description}" }
                }
            }
        }
    }
}

fn calculate_progress_percent(status: &TaskStatus) -> u8 {
    match status {
        TaskStatus::Pending => 0,
        TaskStatus::Queued => 10,
        TaskStatus::Running => 50,
        TaskStatus::Completed => 100,
        TaskStatus::Failed => 100,
        TaskStatus::Paused => 30,
        TaskStatus::Cancelled => 100,
    }
}

fn get_status_info(status: &TaskStatus) -> (&'static str, &'static str) {
    match status {
        TaskStatus::Pending => ("#fbbf24", "待处理"),
        TaskStatus::Queued => ("#60a5fa", "已排队"),
        TaskStatus::Running => ("#3b82f6", "运行中"),
        TaskStatus::Completed => ("#10b981", "已完成"),
        TaskStatus::Failed => ("#ef4444", "失败"),
        TaskStatus::Paused => ("#9ca3af", "已暂停"),
        TaskStatus::Cancelled => ("#6b7280", "已取消"),
    }
}
