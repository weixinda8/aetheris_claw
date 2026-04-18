from typing import Dict, List, Optional, Any
from dataclasses import dataclass
from datetime import datetime

@dataclass
class Task:
    """任务模型"""
    id: str
    description: str
    status: str
    priority: int
    created_at: datetime
    updated_at: datetime
    result: Optional[str] = None
    error: Optional[str] = None

@dataclass
class Agent:
    """Agent 模型"""
    id: str
    name: str
    agent_type: str
    status: str
    capabilities: List[str]
    created_at: datetime

@dataclass
class Skill:
    """技能模型"""
    id: str
    name: str
    description: str
    version: str
    author: str
    capabilities: List[str]
    created_at: datetime
    updated_at: datetime
