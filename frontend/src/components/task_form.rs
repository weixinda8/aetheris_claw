use crate::api::ApiClient;
use crate::models::{CreateTaskRequest, Task};
use dioxus::prelude::*;

#[component]
pub fn TaskForm(
    on_task_created: EventHandler<Task>,
) -> Element {
    let mut title = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut priority = use_signal(|| String::from("medium"));
    let is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

    let on_submit = move |_evt: Event<FormData>| {
        if title.read().is_empty() {
            error_message.set(Some("请输入任务标题".to_string()));
            return;
        }

        spawn({
            let mut title = title.clone();
            let mut description = description.clone();
            let mut priority = priority.clone();
            let on_task_created = on_task_created.clone();
            let mut is_submitting = is_submitting.clone();
            let mut error_message = error_message.clone();

            async move {
                is_submitting.set(true);
                error_message.set(None);

                let request = CreateTaskRequest {
                    title: title.read().clone(),
                    description: description.read().clone(),
                    priority: priority.read().clone(),
                };

                match ApiClient::create_task(request).await {
                    Ok(task) => {
                        title.set(String::new());
                        description.set(String::new());
                        priority.set(String::from("medium"));
                        on_task_created.call(task);
                    }
                    Err(e) => {
                        error_message.set(Some(e));
                    }
                }

                is_submitting.set(false);
            }
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 max-w-2xl mx-auto",
            h2 { class: "text-2xl font-bold text-gray-800 mb-6", "提交新任务" }

            if let Some(error) = error_message.read().as_ref() {
                div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                    "{error}"
                }
            }

            form { onsubmit: on_submit,
                div { class: "mb-4",
                    label { class: "block text-gray-700 text-sm font-bold mb-2", "任务标题 *" }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: "{title}",
                        placeholder: "输入任务标题...",
                        disabled: *is_submitting.read(),
                        oninput: move |evt| title.set(evt.value()),
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-gray-700 text-sm font-bold mb-2", "任务描述" }
                    textarea {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        rows: 4,
                        value: "{description}",
                        placeholder: "输入任务详细描述...",
                        disabled: *is_submitting.read(),
                        oninput: move |evt| description.set(evt.value()),
                    }
                }

                div { class: "mb-6",
                    label { class: "block text-gray-700 text-sm font-bold mb-2", "优先级" }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        value: "{priority}",
                        disabled: *is_submitting.read(),
                        onchange: move |evt| priority.set(evt.value()),
                        option { value: "low", "低" }
                        option { value: "medium", "中" }
                        option { value: "high", "高" }
                        option { value: "urgent", "紧急" }
                    }
                }

                button {
                    class: "w-full bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded-md transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed",
                    r#type: "submit",
                    disabled: *is_submitting.read(),
                    if *is_submitting.read() {
                        "提交中..."
                    } else {
                        "提交任务"
                    }
                }
            }
        }
    }
}
