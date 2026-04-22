# engine/argot/research/signal/phase14/adapters/__init__.py
from argot.research.signal.phase14.adapters.language_adapter import LanguageAdapter
from argot.research.signal.phase14.adapters.python_adapter import PythonAdapter
from argot.research.signal.phase14.adapters.registry import adapter_for_files, get_adapter

__all__ = ["LanguageAdapter", "PythonAdapter", "adapter_for_files", "get_adapter"]
