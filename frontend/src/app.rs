use crate::api::ApiClient;
use crate::components::*;
use crate::models::{Agent, AgentState, AgentType, LoginRequest, Skill, Task, User};
use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Login,
    Home,
    Tasks,
    Agents,
    Skills,
    Dashboard,
    Logs,
    Alerts,
    Intervention,
}

pub fn App() -> Element {
    let mut current_page = use_signal(|| Page::Login);
    let mut refresh_tasks = use_signal(|| false);
    let mut selected_task_id = use_signal(|| Option::<Uuid>::None);
    let mut user = use_signal(|| Option::<User>::None);
    let mut login_error = use_signal(|| Option::<String>::None);

    let on_task_created = move |_task: Task| {
        refresh_tasks.set(true);
    };

    let on_view_task_details = move |task_id: Uuid| {
        selected_task_id.set(Some(task_id));
        current_page.set(Page::Tasks);
    };

    let on_login = move |(username, password): (String, String)| {
        spawn(async move {
            let request = LoginRequest { username, password };
            match ApiClient::login(request).await {
                Ok(response) => {
                    user.set(Some(response.user));
                    current_page.set(Page::Home);
                    login_error.set(None);
                },
                Err(error) => {
                    login_error.set(Some(error));
                }
            }
        });
    };

    let on_logout = move || {
        spawn(async move {
            let _ = ApiClient::logout().await;
            user.set(None);
            current_page.set(Page::Login);
        });
    };

    rsx! {
        div { class: "min-h-screen bg-gray-50",
            match *user.read() {
                Some(_) => rsx! {
                    nav { class: "bg-gray-800 shadow-lg",
                        div { class: "max-w-7xl mx-auto px-4",
                            div { class: "flex items-center justify-between h-16",
                                div { class: "flex items-center",
                                    h1 { class: "text-white text-xl font-bold", "Aetheris 数字军团指挥中枢" }
                                }
                                div { class: "flex space-x-1 overflow-x-auto",
                                    NavButton {
                                        page: Page::Home,
                                        current_page,
                                        label: "首页",
                                    }
                                    NavButton {
                                        page: Page::Tasks,
                                        current_page,
                                        label: "任务管理",
                                    }
                                    NavButton {
                                        page: Page::Agents,
                                        current_page,
                                        label: "Agent管理",
                                    }
                                    NavButton {
                                        page: Page::Skills,
                                        current_page,
                                        label: "技能管理",
                                    }
                                    NavButton {
                                        page: Page::Dashboard,
                                        current_page,
                                        label: "系统指标",
                                    }
                                    NavButton {
                                        page: Page::Logs,
                                        current_page,
                                        label: "日志告警",
                                    }
                                    NavButton {
                                        page: Page::Alerts,
                                        current_page,
                                        label: "告警配置",
                                    }
                                    NavButton {
                                        page: Page::Intervention,
                                        current_page,
                                        label: "人工干预",
                                    }
                                    button {
                                        class: "px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-700 transition-colors whitespace-nowrap",
                                        onclick: move |_| on_logout(),
                                        "退出登录"
                                    }
                                }
                            }
                        }
                    }

                    main { class: "max-w-7xl mx-auto px-4 py-8",
                        match *current_page.read() {
                            Page::Home => rsx! { HomePage { on_task_created } },
                            Page::Tasks => rsx! { TasksPage { on_view_task_details } },
                            Page::Agents => rsx! { AgentsPage {} },
                            Page::Skills => rsx! { SkillsPage {} },
                            Page::Dashboard => rsx! { DashboardPage {} },
                            Page::Logs => rsx! { LogsPage {} },
                            Page::Alerts => rsx! { AlertsPage {} },
                            Page::Intervention => rsx! { InterventionPage {} },
                            _ => rsx! { HomePage { on_task_created } },
                        }
                    }
                },
                None => rsx! {
                    LoginPage { on_login, error: login_error.read().clone() }
                }
            }
        }
    }
}

#[component]
fn NavButton(
    page: Page,
    current_page: Signal<Page>,
    label: &'static str,
) -> Element {
    let is_active = *current_page.read() == page;

    rsx! {
        button {
            class: if is_active {
                "px-3 py-2 rounded-md text-sm font-medium text-white bg-gray-900 whitespace-nowrap"
            } else {
                "px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-700 transition-colors whitespace-nowrap"
            },
            onclick: move |_| current_page.set(page),
            {label}
        }
    }
}

#[component]
fn HomePage(on_task_created: EventHandler<Task>) -> Element {
    rsx! {
        div {
            div { class: "text-center mb-12",
                h2 { class: "text-4xl font-bold text-gray-800 mb-4", "欢迎使用 Aetheris" }
                p { class: "text-xl text-gray-600", "AI 原生、自进化、分布式、全链路可信的复杂任务执行操作系统" }
            }

            div { class: "grid md:grid-cols-2 gap-8 mb-12",
                div { class: "bg-white rounded-lg shadow-md p-6",
                    h3 { class: "text-xl font-semibold text-gray-800 mb-4", "快速开始" }
                    TaskForm { on_task_created }
                }

                div { class: "bg-white rounded-lg shadow-md p-6",
                    h3 { class: "text-xl font-semibold text-gray-800 mb-4", "系统概览" }
                    SystemOverview {}
                }
            }

            div { class: "grid md:grid-cols-3 gap-6",
                QuickLinkCard {
                    title: "任务管理",
                    description: "查看所有任务，监控执行状态",
                    icon: "📋",
                    color: "#3b82f6"
                }
                QuickLinkCard {
                    title: "系统指标",
                    description: "查看系统运行状态和性能指标",
                    icon: "📊",
                    color: "#8b5cf6"
                }
                QuickLinkCard {
                    title: "日志告警",
                    description: "查看审计日志和系统告警",
                    icon: "📝",
                    color: "#10b981"
                }
            }
        }
    }
}

#[component]
fn QuickLinkCard(title: &'static str, description: &'static str, icon: &'static str, color: &'static str) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 cursor-pointer hover:shadow-lg transition-shadow",
            div { class: "flex items-center gap-4 mb-4",
                span { class: "text-4xl", "{icon}" }
                div {
                    h4 { class: "text-xl font-semibold text-gray-800", "{title}" }
                }
            }
            p { class: "text-gray-600", "{description}" }
        }
    }
}

#[component]
fn SystemOverview() -> Element {
    rsx! {
        div { class: "space-y-4",
            div { class: "flex justify-between items-center p-4 bg-blue-50 rounded-lg",
                span { class: "text-gray-700", "任务总数" }
                span { class: "text-2xl font-bold text-blue-600", "--" }
            }
            div { class: "flex justify-between items-center p-4 bg-green-50 rounded-lg",
                span { class: "text-gray-700", "运行中任务" }
                span { class: "text-2xl font-bold text-green-600", "--" }
            }
            div { class: "flex justify-between items-center p-4 bg-purple-50 rounded-lg",
                span { class: "text-gray-700", "在线 Agent" }
                span { class: "text-2xl font-bold text-purple-600", "--" }
            }
            div { class: "flex justify-between items-center p-4 bg-yellow-50 rounded-lg",
                span { class: "text-gray-700", "系统运行时间" }
                span { class: "text-2xl font-bold text-yellow-600", "--" }
            }
        }
    }
}

#[component]
fn TasksPage(on_view_task_details: EventHandler<Uuid>) -> Element {
    rsx! {
        div { class: "space-y-8",
            TaskList { on_view_task_details }
        }
    }
}

#[component]
fn DashboardPage() -> Element {
    rsx! {
        div {
            MetricsDashboard {}
        }
    }
}

#[component]
fn LogsPage() -> Element {
    rsx! {
        div {
            LogsViewer {}
        }
    }
}

#[component]
fn AlertsPage() -> Element {
    rsx! {
        div {
            AlertsConfig {}
        }
    }
}

#[component]
fn InterventionPage() -> Element {
    rsx! {
        div {
            HumanIntervention {}
        }
    }
}

#[component]
fn LoginPage(on_login: EventHandler<(String, String)>, error: Option<String>) -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());

    rsx! {
        div { class: "flex items-center justify-center min-h-screen bg-gray-50",
            div { class: "w-full max-w-md p-8 space-y-6 bg-white rounded-lg shadow-md",
                div { class: "text-center",
                    h2 { class: "text-3xl font-bold text-gray-900", "Aetheris 登录" }
                    p { class: "mt-2 text-sm text-gray-600", "请输入您的用户名和密码" }
                }

                if let Some(err) = error {
                    div { class: "p-3 mb-4 text-sm text-red-700 bg-red-100 rounded-md",
                        "{err}"
                    }
                }

                form { 
                    onsubmit: move |e| {
                        e.prevent_default();
                        on_login(username.read().clone(), password.read().clone());
                    },
                    class: "space-y-4",

                    div {
                        label { 
                            class: "block text-sm font-medium text-gray-700",
                            r#for: "username",
                            "用户名"
                        }
                        input {
                            id: "username",
                            r#type: "text",
                            class: "w-full px-3 py-2 mt-1 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: username.read(),
                            oninput: move |e| username.set(e.value.clone()),
                            required: true,
                        }
                    }

                    div {
                        label { 
                            class: "block text-sm font-medium text-gray-700",
                            r#for: "password",
                            "密码"
                        }
                        input {
                            id: "password",
                            r#type: "password",
                            class: "w-full px-3 py-2 mt-1 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: password.read(),
                            oninput: move |e| password.set(e.value.clone()),
                            required: true,
                        }
                    }

                    div {
                        button {
                            type: "submit",
                            class: "w-full px-4 py-2 font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                            "登录"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AgentsPage() -> Element {
    let agents = use_resource(move || async move {
        ApiClient::get_agents().await
    });

    rsx! {
        div { class: "space-y-8",
            div { class: "flex justify-between items-center",
                h2 { class: "text-2xl font-bold text-gray-800", "Agent管理" }
                button { 
                    class: "px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors",
                    "创建Agent"
                }
            }
            
            if agents.read().is_none() {
                rsx! {
                    div { class: "flex justify-center py-12",
                        div { class: "animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500" }
                    }
                }
            } else if let Some(Ok(agents_list)) = agents.read().as_ref() {
                if agents_list.is_empty() {
                    rsx! {
                        div { class: "flex flex-col items-center justify-center py-12 bg-gray-50 rounded-lg",
                            span { class: "text-6xl mb-4", "🤖" }
                            h3 { class: "text-xl font-semibold text-gray-700 mb-2", "暂无Agent" }
                            p { class: "text-gray-600 text-center", "点击上方按钮创建您的第一个Agent" }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "grid md:grid-cols-2 lg:grid-cols-3 gap-6",
                            for agent in agents_list {
                                AgentCard { agent: agent.clone() }
                            }
                        }
                    }
                }
            } else if let Some(Err(error)) = agents.read().as_ref() {
                rsx! {
                    div { class: "p-4 text-red-700 bg-red-100 rounded-md",
                        "获取Agent列表失败: {error}"
                    }
                }
            } else {
                rsx! {
                    div { class: "flex justify-center py-12",
                        div { class: "animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500" }
                    }
                }
            }
        }
    }
}

#[component]
fn AgentCard(agent: Agent) -> Element {
    let agent_type_str = match agent.agent_type {
        AgentType::Code => "代码",
        AgentType::Data => "数据",
        AgentType::Ops => "运维",
        AgentType::Office => "办公",
        AgentType::Industrial => "工业",
        AgentType::Compliance => "合规",
    };

    let state_str = match agent.state {
        AgentState::Idle => "空闲",
        AgentState::Busy => "忙碌",
        AgentState::Paused => "暂停",
        AgentState::Error => "错误",
    };

    let state_color = match agent.state {
        AgentState::Idle => "text-green-600",
        AgentState::Busy => "text-blue-600",
        AgentState::Paused => "text-yellow-600",
        AgentState::Error => "text-red-600",
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow",
            div { class: "flex justify-between items-start mb-4",
                div {
                    h3 { class: "text-xl font-semibold text-gray-800", "{agent.name}" }
                    p { class: "text-sm text-gray-600", "类型: {agent_type_str}" }
                }
                span { class: "px-2 py-1 text-xs font-medium rounded-full {state_color} bg-gray-100",
                    "{state_str}"
                }
            }
            
            if let Some(desc) = &agent.description {
                p { class: "text-gray-600 mb-4", "{desc}" }
            }
            
            div { class: "flex space-x-2",
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700",
                    "启动"
                }
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-yellow-600 rounded-md hover:bg-yellow-700",
                    "停止"
                }
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-gray-600 rounded-md hover:bg-gray-700",
                    "详情"
                }
            }
        }
    }
}

#[component]
fn SkillsPage() -> Element {
    let skills = use_resource(move || async move {
        ApiClient::get_skills().await
    });

    rsx! {
        div { class: "space-y-8",
            div { class: "flex justify-between items-center",
                h2 { class: "text-2xl font-bold text-gray-800", "技能管理" }
                button { 
                    class: "px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors",
                    "创建技能"
                }
            }
            
            if skills.read().is_none() {
                rsx! {
                    div { class: "flex justify-center py-12",
                        div { class: "animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500" }
                    }
                }
            } else if let Some(Ok(skills_list)) = skills.read().as_ref() {
                if skills_list.is_empty() {
                    rsx! {
                        div { class: "flex flex-col items-center justify-center py-12 bg-gray-50 rounded-lg",
                            span { class: "text-6xl mb-4", "⚙️" }
                            h3 { class: "text-xl font-semibold text-gray-700 mb-2", "暂无技能" }
                            p { class: "text-gray-600 text-center", "点击上方按钮创建您的第一个技能" }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "grid md:grid-cols-2 lg:grid-cols-3 gap-6",
                            for skill in skills_list {
                                SkillCard { skill: skill.clone() }
                            }
                        }
                    }
                }
            } else if let Some(Err(error)) = skills.read().as_ref() {
                rsx! {
                    div { class: "p-4 text-red-700 bg-red-100 rounded-md",
                        "获取技能列表失败: {error}"
                    }
                }
            } else {
                rsx! {
                    div { class: "flex justify-center py-12",
                        div { class: "animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500" }
                    }
                }
            }
        }
    }
}

#[component]
fn SkillCard(skill: Skill) -> Element {
    let status_str = if skill.is_enabled { "启用" } else { "禁用" };
    let status_color = if skill.is_enabled { "text-green-600" } else { "text-gray-600" };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow",
            div { class: "flex justify-between items-start mb-4",
                div {
                    h3 { class: "text-xl font-semibold text-gray-800", "{skill.name}" }
                    p { class: "text-sm text-gray-600", "版本: {skill.version}" }
                    p { class: "text-sm text-gray-600", "分类: {skill.category}" }
                }
                span { class: "px-2 py-1 text-xs font-medium rounded-full {status_color} bg-gray-100",
                    "{status_str}"
                }
            }
            
            p { class: "text-gray-600 mb-4", "{skill.description}" }
            
            div { class: "flex space-x-2",
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700",
                    "编辑"
                }
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-yellow-600 rounded-md hover:bg-yellow-700",
                    "{if skill.is_enabled { '禁用' } else { '启用' }}"
                }
                button { 
                    class: "px-3 py-1 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700",
                    "删除"
                }
            }
        }
    }
}
