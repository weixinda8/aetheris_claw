#!/usr/bin/env python3
"""
测试Aetheris技能市场API
"""

import requests
import json
import uuid
import time

BASE_URL = "http://localhost:8080/api"

class SkillMarketplaceTest:
    def __init__(self):
        self.base_url = BASE_URL
        self.session = requests.Session()
        self.test_skill_id = None
    
    def test_health(self):
        """测试健康检查端点"""
        print("测试健康检查...")
        response = self.session.get(f"{self.base_url}/health")
        assert response.status_code == 200, f"健康检查失败: {response.status_code}"
        data = response.json()
        assert data["status"] == "ok", f"健康状态异常: {data}"
        print("✓ 健康检查通过")
    
    def test_stats(self):
        """测试统计信息端点"""
        print("测试统计信息...")
        response = self.session.get(f"{self.base_url}/stats")
        assert response.status_code == 200, f"统计信息获取失败: {response.status_code}"
        data = response.json()
        assert "total_skills" in data, "统计信息缺少total_skills字段"
        assert "published_skills" in data, "统计信息缺少published_skills字段"
        assert "total_downloads" in data, "统计信息缺少total_downloads字段"
        print("✓ 统计信息获取通过")
    
    def test_create_skill(self):
        """测试创建技能"""
        print("测试创建技能...")
        skill_data = {
            "skill_id": f"test-skill-{uuid.uuid4()}",
            "name": "测试技能",
            "description": "这是一个测试技能",
            "long_description": "这是一个详细的测试技能描述",
            "version": "1.0.0",
            "category": "测试",
            "categories": ["测试", "工具"],
            "tags": ["test", "demo"],
            "call_mode": "Text",
            "permission_level": "Public",
            "priority": "Medium",
            "required_permissions": [],
            "input_schema": {"type": "object", "properties": {}}, 
            "output_schema": {"type": "object", "properties": {}}, 
            "example_input": {},
            "example_output": {},
            "dependencies": [],
            "metadata": {},
            "content": {"name": "测试技能", "description": "测试技能内容"},
            "changelog": "初始版本"
        }
        
        response = self.session.post(f"{self.base_url}/skills", json=skill_data)
        assert response.status_code == 200, f"创建技能失败: {response.status_code}"
        data = response.json()
        assert "skill_id" in data, "创建技能响应缺少skill_id字段"
        assert "version" in data, "创建技能响应缺少version字段"
        self.test_skill_id = data["skill_id"]
        print(f"✓ 创建技能通过，技能ID: {self.test_skill_id}")
    
    def test_get_skill(self):
        """测试获取技能详情"""
        if not self.test_skill_id:
            print("跳过获取技能详情测试，因为没有测试技能ID")
            return
        
        print("测试获取技能详情...")
        response = self.session.get(f"{self.base_url}/skills/{self.test_skill_id}")
        assert response.status_code == 200, f"获取技能详情失败: {response.status_code}"
        data = response.json()
        assert data["id"] == self.test_skill_id, "获取的技能ID与创建的不一致"
        print("✓ 获取技能详情通过")
    
    def test_list_skills(self):
        """测试获取技能列表"""
        print("测试获取技能列表...")
        response = self.session.get(f"{self.base_url}/skills")
        assert response.status_code == 200, f"获取技能列表失败: {response.status_code}"
        data = response.json()
        assert "skills" in data, "技能列表响应缺少skills字段"
        assert "total" in data, "技能列表响应缺少total字段"
        print(f"✓ 获取技能列表通过，共 {data['total']} 个技能")
    
    def test_download_skill(self):
        """测试下载技能"""
        if not self.test_skill_id:
            print("跳过下载技能测试，因为没有测试技能ID")
            return
        
        print("测试下载技能...")
        response = self.session.get(f"{self.base_url}/skills/{self.test_skill_id}/download")
        assert response.status_code == 200, f"下载技能失败: {response.status_code}"
        data = response.json()
        assert data["skill_id"] == self.test_skill_id, "下载的技能ID与创建的不一致"
        assert "content" in data, "下载技能响应缺少content字段"
        print("✓ 下载技能通过")
    
    def test_create_skill_version(self):
        """测试创建技能版本"""
        if not self.test_skill_id:
            print("跳创建技能版本测试，因为没有测试技能ID")
            return
        
        print("测试创建技能版本...")
        version_data = {
            "version": "1.1.0",
            "content": {"name": "测试技能", "description": "测试技能内容 v1.1.0"},
            "changelog": "更新了技能内容"
        }
        
        response = self.session.post(f"{self.base_url}/skills/{self.test_skill_id}/versions", json=version_data)
        assert response.status_code == 200, f"创建技能版本失败: {response.status_code}"
        data = response.json()
        assert "version_id" in data, "创建技能版本响应缺少version_id字段"
        assert "skill_id" in data, "创建技能版本响应缺少skill_id字段"
        assert "version" in data, "创建技能版本响应缺少version字段"
        assert data["version"] == "1.1.0", "创建的版本号与预期不一致"
        print("✓ 创建技能版本通过")
    
    def test_list_skill_versions(self):
        """测试获取技能版本列表"""
        if not self.test_skill_id:
            print("跳过获取技能版本列表测试，因为没有测试技能ID")
            return
        
        print("测试获取技能版本列表...")
        response = self.session.get(f"{self.base_url}/skills/{self.test_skill_id}/versions")
        assert response.status_code == 200, f"获取技能版本列表失败: {response.status_code}"
        data = response.json()
        assert isinstance(data, list), "技能版本列表应该是一个数组"
        assert len(data) >= 1, "技能版本列表至少应该有一个版本"
        print(f"✓ 获取技能版本列表通过，共 {len(data)} 个版本")
    
    def test_create_review(self):
        """测试创建技能评论"""
        if not self.test_skill_id:
            print("跳过创建技能评论测试，因为没有测试技能ID")
            return
        
        print("测试创建技能评论...")
        review_data = {
            "skill_id": self.test_skill_id,
            "rating": 5,
            "title": "很好的技能",
            "content": "这是一个非常好用的技能，推荐给大家！"
        }
        
        response = self.session.post(f"{self.base_url}/skills/{self.test_skill_id}/reviews", json=review_data)
        assert response.status_code == 200, f"创建技能评论失败: {response.status_code}"
        print("✓ 创建技能评论通过")
    
    def test_list_reviews(self):
        """测试获取技能评论列表"""
        if not self.test_skill_id:
            print("跳过获取技能评论列表测试，因为没有测试技能ID")
            return
        
        print("测试获取技能评论列表...")
        response = self.session.get(f"{self.base_url}/skills/{self.test_skill_id}/reviews")
        assert response.status_code == 200, f"获取技能评论列表失败: {response.status_code}"
        data = response.json()
        assert "reviews" in data, "评论列表响应缺少reviews字段"
        assert "total" in data, "评论列表响应缺少total字段"
        print(f"✓ 获取技能评论列表通过，共 {data['total']} 条评论")
    
    def test_run_all(self):
        """运行所有测试"""
        print("=" * 60)
        print("开始测试Aetheris技能市场API")
        print("=" * 60)
        
        try:
            self.test_health()
            self.test_stats()
            self.test_create_skill()
            self.test_get_skill()
            self.test_list_skills()
            self.test_download_skill()
            self.test_create_skill_version()
            self.test_list_skill_versions()
            self.test_create_review()
            self.test_list_reviews()
            
            print("=" * 60)
            print("所有测试通过！")
            print("=" * 60)
        except Exception as e:
            print(f"测试失败: {e}")
            raise

if __name__ == "__main__":
    test = SkillMarketplaceTest()
    test.run_all()
