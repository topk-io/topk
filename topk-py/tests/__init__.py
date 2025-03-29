import os
import random
from dataclasses import dataclass

from topk_sdk import Client


@dataclass
class ProjectContext:
    client: Client
    scope_prefix: str

    def scope(self, name: str):
        return f"{self.scope_prefix}-{name}"


def new_project_context():
    TOPK_API_KEY = os.environ["TOPK_API_KEY"].splitlines()[0].strip()
    TOPK_HOST = os.environ.get("TOPK_HOST", "topk.io")
    TOPK_REGION = os.environ.get("TOPK_REGION", "elastica")
    TOPK_HTTPS = os.environ.get("TOPK_HTTPS", "false") == "true"

    print(
        dict(
            api_key=TOPK_API_KEY,
            region=TOPK_REGION,
            host=TOPK_HOST,
            https=TOPK_HTTPS,
        )
    )

    client = Client(
        api_key=TOPK_API_KEY,
        region=TOPK_REGION,
        host=TOPK_HOST,
        https=TOPK_HTTPS,
    )

    return ProjectContext(
        scope_prefix="%s" % str(random.random()).replace(".", ""),
        client=client,
    )
