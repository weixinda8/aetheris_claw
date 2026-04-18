use crate::api::ApiClient;
use crate::models::{Task, TaskStatus};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn TaskList(on_view_task_details: EventHandler<Uuid>) -> Element {
    let tasks = use_signal(|| Vec::<Task>::new());
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_tasks = move || {
        spawn({
            let mut tasks = tasks.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            async move {
                is_loading.set(true);
                error.set(None);
                match ApiClient::get_tasks().await {
                    Ok(fetched_tasks) => {
                        tasks.set(fetched_tasks);
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
        div { class: "max-w-6xl mx-auto",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "任务列表" }
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
                    p { "加载任务失败: {err}" }
                    button {
                        class: "mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md",
                        onclick: move |_| load_tasks(),
                        "重试"
                    }
                }
            } else if tasks.read().is_empty() {
                div { class: "text-center py-12 bg-white rounded-lg shadow-md",
                    p { class: "text-gray-600", "暂无任务" }
                }
            } else {
                div { class: "space-y-4",
                    for task in tasks.read().iter() {
                        TaskItem { 
                            task: task.clone(),
                            on_view_details: on_view_task_details.clone()
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskItem(task: Task, on_view_details: EventHandler<Uuid>) -> Element {
    let (status_class, status_text) = get_status_info(&task.status);
    let updated_at_str = task.updated_at.format("%Y-%m-%d %H:%M").to_string();
    let created_at_str = task.created_at.format("%Y-%m-%d %H:%M").to_string();

    rsx! {
        div { 
            class: "bg-white rounded-lg shadow-md p-6 cursor-pointer hover:shadow-lg transition-shadow",
            onclick: move |_| on_view_details.call(task.id),
            div { class: "flex justify-between items-start mb-4",
                div { class: "flex-1",
                    h3 { class: "text-xl font-semibold text-gray-800", "{task.title}" }
                    p { class: "text-gray-600 mt-1", "{task.description}" }
                }
                div { class: "flex flex-col items-end gap-2",
                    span { 
                        class: "px-3 py-1 rounded-full text-sm font-medium {status_class}",
                        "{status_text}"
                    }
                    span { class: "px-3 py-1 bg-gray-100 text-gray-800 rounded-full text-sm", "{task.priority}" }
                }
            }
            div { class: "flex justify-between text-sm text-gray-500 pt-4 border-t border-gray-200",
                span { "创建: {created_at_str}" }
                span { "更新: {updated_at_str}" }
            }
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
