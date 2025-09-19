import sys
import types
from .topk_sdk import *

__doc__ = topk_sdk.__doc__ # type: ignore
if hasattr(topk_sdk, "__all__"): # type: ignore
    __all__ = topk_sdk.__all__ # type: ignore

for k, v in vars(topk_sdk).items(): # type: ignore
  if isinstance(v, types.ModuleType):
    sys.modules[f"topk_sdk.{k}"] = v