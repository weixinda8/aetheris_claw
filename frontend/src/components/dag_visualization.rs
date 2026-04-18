use crate::api::ApiClient;
use crate::models::{DAGNode, DAGEdge, NodeStatus, TaskDAG};
use dioxus::prelude::*;
use uuid::Uuid;

fn node_status_color(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "#fbbf24",
        NodeStatus::Running => "#3b82f6",
        NodeStatus::Completed => "#10b981",
        NodeStatus::Failed => "#ef4444",
        NodeStatus::Skipped => "#9ca3af",
    }
}

fn node_status_text(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "待处理",
        NodeStatus::Running => "运行中",
        NodeStatus::Completed => "已完成",
        NodeStatus::Failed => "失败",
        NodeStatus::Skipped => "已跳过",
    }
}

#[component]
pub fn DAGVisualization(task_id: Uuid) -> Element {
    let dag = use_signal(|| Option::<TaskDAG>::None);
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

    let load_dag = move || {
        spawn({
            let mut dag = dag.clone();
            let mut is_loading = is_loading.clone();
            let mut error = error.clone();
            let task_id = task_id;
            async move {
                is_loading.set(true);
                error.set(None);
                match ApiClient::get_task_dag(task_id).await {
                    Ok(fetched_dag) => {
                        dag.set(Some(fetched_dag));
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
        load_dag();
    });

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            div { class: "flex justify-between items-center mb-6",
                h2 { class: "text-2xl font-bold text-gray-800", "执行拓扑图" }
                button {
                    class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors",
                    onclick: move |_| load_dag(),
                    "刷新"
                }
            }

            if *is_loading.read() {
                div { class: "text-center py-12",
                    p { class: "text-gray-600", "加载中..." }
                }
            } else if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                    p { "加载 DAG 失败: {err}" }
                    button {
                        class: "mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md",
                        onclick: move |_| load_dag(),
                        "重试"
                    }
                }
            } else if let Some(dag_data) = dag.read().as_ref() {
                div { class: "space-y-4",
                    NodeList { nodes: dag_data.nodes.clone() }
                    if !dag_data.edges.is_empty() {
                        EdgeList { edges: dag_data.edges.clone(), nodes: dag_data.nodes.clone() }
                    }
                }
            } else {
                div { class: "text-center py-12 bg-white rounded-lg shadow-md",
                    p { class: "text-gray-600", "暂无 DAG 数据" }
                }
            }
        }
    }
}

#[component]
fn NodeList(nodes: Vec<DAGNode>) -> Element {
    rsx! {
        div {
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "节点列表" }
            div { class: "space-y-4",
                for node in nodes.iter() {
                    NodeItem { node: node.clone() }
                }
            }
        }
    }
}

#[component]
fn NodeItem(node: DAGNode) -> Element {
    let color = node_status_color(&node.status);
    let status_text = node_status_text(&node.status);

    rsx! {
        div {
            class: "p-4 rounded-lg border border-gray-300",
            style: "background-color: {color}20;",
            div { class: "flex justify-between items-start",
                div {
                    h4 { class: "font-semibold text-gray-900", "{node.name}" }
                    if let Some(desc) = &node.description {
                        p { class: "text-sm text-gray-600 mt-1", "{desc}" }
                    }
                }
                span {
                    class: "px-2 py-1 rounded-full text-xs font-medium text-white",
                    style: "background-color: {color};",
                    "{status_text}"
                }
            }
        }
    }
}

#[component]
fn EdgeList(edges: Vec<DAGEdge>, nodes: Vec<DAGNode>) -> Element {
    rsx! {
        div { class: "mt-6 pt-4 border-t border-gray-200",
            h3 { class: "text-xl font-semibold text-gray-800 mb-4", "依赖关系" }
            div { class: "space-y-2",
                for edge in edges.iter() {
                    EdgeItem { edge: edge.clone(), nodes: nodes.clone() }
                }
            }
        }
    }
}

#[component]
fn EdgeItem(edge: DAGEdge, nodes: Vec<DAGNode>) -> Element {
    let from_node = nodes.iter().find(|n| n.id == edge.from_node);
    let to_node = nodes.iter().find(|n| n.id == edge.to_node);

    let from_name = from_node.map(|n| n.name.clone()).unwrap_or_else(|| format!("{}", edge.from_node));
    let to_name = to_node.map(|n| n.name.clone()).unwrap_or_else(|| format!("{}", edge.to_node));

    rsx! {
        div { class: "flex items-center gap-2 text-sm text-gray-600",
            span { class: "font-medium", "{from_name}" }
            span { "→" }
            span { class: "font-medium", "{to_name}" }
            span { class: "text-xs text-gray-400", "({edge.dependency_type})" }
        }
    }
}
