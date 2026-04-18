import requests
import json
from typing import Dict, List, Optional, Any
from .models import Task, Agent, Skill
from .exceptions import AetherisError

class AetherisClient:
    """Aetheris API 客户端"""
    
    def __init__(self, base_url: str = "http://localhost:3000"):
        """初始化客户端
        
        Args:
            base_url: Aetheris 服务的基础 URL
        """
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        })
    
    def _request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """发送 HTTP 请求
        
        Args:
            method: HTTP 方法 (GET, POST, PUT, DELETE)
            endpoint: API 端点
            **kwargs: 传递给 requests 的其他参数
            
        Returns:
            响应数据的字典
            
        Raises:
            AetherisError: API 请求失败时
        """
        url = f"{self.base_url}{endpoint}"
        try:
            response = self.session.request(method, url, **kwargs)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            raise AetherisError(f"API 请求失败: {str(e)}")
    
    def create_task(self, description: str, priority: int = 5) -> Dict[str, Any]:
        """创建新任务
        
        Args:
            description: 任务描述
            priority: 任务优先级 (1-10)
            
        Returns:
            包含任务信息的字典
        """
        data = {
            "description": description,
            "priority": priority
        }
        return self._request("POST", "/api/tasks", json=data)
    
    def get_task(self, task_id: str) -> Task:
        """获取任务详情
        
        Args:
            task_id: 任务 ID
            
        Returns:
            Task 对象
        """
        data = self._request("GET", f"/api/tasks/{task_id}")
        return Task(**data)
    
    def get_task_status(self, task_id: str) -> str:
        """获取任务状态
        
        Args:
            task_id: 任务 ID
            
        Returns:
            任务状态字符串
        """
        task = self.get_task(task_id)
        return task.status
    
    def list_tasks(self, status: Optional[str] = None) -> List[Task]:
        """列出所有任务
        
        Args:
            status: 可选的状态过滤
            
        Returns:
            Task 对象列表
        """
        params = {}
        if status:
            params["status"] = status
        data = self._request("GET", "/api/tasks", params=params)
        return [Task(**task) for task in data]
    
    def stop_task(self, task_id: str) -> Dict[str, Any]:
        """停止任务
        
        Args:
            task_id: 任务 ID
            
        Returns:
            操作结果
        """
        return self._request("POST", f"/api/tasks/{task_id}/stop")
    
    def list_agents(self) -> List[Agent]:
        """列出所有 Agent
        
        Returns:
            Agent 对象列表
        """
        data = self._request("GET", "/api/agents")
        return [Agent(**agent) for agent in data]
    
    def get_agent(self, agent_id: str) -> Agent:
        """获取 Agent 详情
        
        Args:
            agent_id: Agent ID
            
        Returns:
            Agent 对象
        """
        data = self._request("GET", f"/api/agents/{agent_id}")
        return Agent(**data)
    
    def list_skills(self) -> List[Skill]:
        """列出所有技能
        
        Returns:
            Skill 对象列表
        """
        data = self._request("GET", "/api/skills")
        return [Skill(**skill) for skill in data]
    
    def get_skill(self, skill_id: str) -> Skill:
        """获取技能详情
        
        Args:
            skill_id: 技能 ID
            
        Returns:
            Skill 对象
        """
        data = self._request("GET", f"/api/skills/{skill_id}")
        return Skill(**data)
