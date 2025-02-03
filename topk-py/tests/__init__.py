import os
import random
from dataclasses import dataclass

from topk_sdk import Client


@dataclass
class ProjectContext:
    org_id: int
    project_id: int
    client: Client
    scope_prefix: str

    def scope(self, name: str):
        return f"{self.scope_prefix}-{name}"


def new_project_context():
    TOPK_API_KEY = os.environ["TOPK_API_KEY"].splitlines()[0].strip()
    client = Client(api_key=TOPK_API_KEY, region="dev", host="ddb:80")

    return ProjectContext(
        org_id=1,
        project_id=1,
        scope_prefix="%s" % str(random.random()).replace(".", ""),
        client=client,
    )
