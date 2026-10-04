import openai

from worker.client import CriterionResult, PairwiseVerdict
from worker.judge_prompts import (
    build_pairwise_judge_prompt,
    build_rubric_judge_prompt,
    parse_pairwise_judge_response,
    parse_rubric_judge_response,
)
from worker.rubric import Rubric


class OpenAIClient:
    """Real Client implementation backed by the OpenAI API. Resolves OPENAI_API_KEY from the environment via the SDK's default credential chain."""

    def __init__(self):
        self._client = openai.OpenAI()

    def _complete(self, model: str, prompt: str) -> str:
        response = self._client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.choices[0].message.content or ""

    def generate(self, model: str, prompt: str) -> str:
        return self._complete(model, prompt)

    def judge_rubric(self, model: str, prompt: str, response: str, rubric: Rubric) -> dict[str, CriterionResult]:
        judge_prompt = build_rubric_judge_prompt(prompt, response, rubric)
        raw = self._complete(model, judge_prompt)
        return parse_rubric_judge_response(raw, rubric)

    def judge_pairwise(
        self,
        model: str,
        prompt: str,
        response_first: str,
        response_second: str,
        identity_first: str | None = None,
        identity_second: str | None = None,
    ) -> PairwiseVerdict:
        judge_prompt = build_pairwise_judge_prompt(prompt, response_first, response_second, identity_first, identity_second)
        raw = self._complete(model, judge_prompt)
        return parse_pairwise_judge_response(raw)
