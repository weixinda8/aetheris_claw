class AetherisError(Exception):
    """Aetheris 基础异常"""
    pass

class AetherisApiError(AetherisError):
    """API 错误异常"""
    pass

class AetherisAuthError(AetherisError):
    """认证错误异常"""
    pass

class AetherisTaskError(AetherisError):
    """任务错误异常"""
    pass
