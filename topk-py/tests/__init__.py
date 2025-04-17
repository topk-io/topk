import os
from dataclasses import dataclass
from uuid import uuid4

from topk_sdk import Client


@dataclass
class ProjectContext:
    client: Client
    scope_prefix: str

    def scope(self, name: str):
        return f"{self.scope_prefix}-{name}"

    def cleanup(self):
        for c in self.client.collections().list():
            if c.name.startswith(self.scope_prefix):
                self.client.collections().delete(c.name)


def new_project_context():
    TOPK_API_KEY = os.environ["TOPK_API_KEY"].splitlines()[0].strip()
    TOPK_HOST = os.environ.get("TOPK_HOST", "topk.io")
    TOPK_REGION = os.environ.get("TOPK_REGION", "elastica")
    TOPK_HTTPS = os.environ.get("TOPK_HTTPS", "true") == "true"

    client = Client(
        api_key=TOPK_API_KEY,
        region=TOPK_REGION,
        host=TOPK_HOST,
        https=TOPK_HTTPS,
    )

    return ProjectContext(
        scope_prefix=f"topk-py-{uuid4()}",
        client=client,
    )
