from typing import Callable

from worker.client import Client, CriterionResult, PairwiseVerdict
from worker.rubric import Rubric


class RoutingClient:
    """Implements Client by dispatching to a real provider client chosen by the model name's prefix. Provider clients are constructed lazily, on first use - some providers' SDKs raise immediately if their API key env var is unset, so a run that never touches a given provider must never construct its client."""

    def __init__(self, factories_by_prefix: dict[str, Callable[[], Client]]):
        self._factories_by_prefix = factories_by_prefix
        self._instances: dict[str, Client] = {}

    def _client_for(self, model: str) -> Client:
        for prefix, factory in self._factories_by_prefix.items():
            if model.startswith(prefix):
                if prefix not in self._instances:
                    self._instances[prefix] = factory()
                return self._instances[prefix]
        raise ValueError(f"no client registered for model '{model}' (known prefixes: {list(self._factories_by_prefix)})")

    def generate(self, model: str, prompt: str) -> str:
        return self._client_for(model).generate(model, prompt)

    def judge_rubric(self, model: str, prompt: str, response: str, rubric: Rubric) -> dict[str, CriterionResult]:
        return self._client_for(model).judge_rubric(model, prompt, response, rubric)

    def judge_pairwise(
        self,
        model: str,
        prompt: str,
        response_first: str,
        response_second: str,
        identity_first: str | None = None,
        identity_second: str | None = None,
    ) -> PairwiseVerdict:
        return self._client_for(model).judge_pairwise(
            model, prompt, response_first, response_second, identity_first, identity_second
        )


def default_routing_client() -> RoutingClient:
    from worker.anthropic_client import AnthropicClient
    from worker.openai_client import OpenAIClient

    return RoutingClient({
        "claude": AnthropicClient,
        "gpt": OpenAIClient,
    })
