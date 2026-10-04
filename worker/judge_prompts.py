import json
import re

from worker.client import CriterionResult, PairwiseVerdict
from worker.rubric import Rubric


def build_rubric_judge_prompt(prompt: str, response: str, rubric: Rubric) -> str:
    criteria_desc = "\n".join(f"- {c.name}: {c.description}" for c in rubric.criteria)
    criterion_names = ", ".join(f'"{c.name}"' for c in rubric.criteria)
    return f"""You are judging a model's response to a prompt, scoring it against a rubric.

PROMPT:
{prompt}

RESPONSE TO JUDGE:
{response}

RUBRIC (scale: {rubric.scale}):
{criteria_desc}

Score the response on each criterion. Respond with ONLY a JSON object, no other text, in this exact shape:
{{{", ".join(f'"{c.name}": {{"score": <int>, "rationale": "<short reason>"}}' for c in rubric.criteria)}}}

The keys must be exactly: {criterion_names}."""


def parse_rubric_judge_response(raw: str, rubric: Rubric) -> dict[str, CriterionResult]:
    data = _extract_json(raw)
    results = {}
    for criterion in rubric.criteria:
        if criterion.name not in data:
            raise ValueError(f"judge response missing criterion '{criterion.name}': {raw}")
        entry = data[criterion.name]
        results[criterion.name] = CriterionResult(
            score=int(entry["score"]),
            rationale=str(entry["rationale"]),
        )
    return results


def build_pairwise_judge_prompt(
    prompt: str,
    response_first: str,
    response_second: str,
    identity_first: str | None,
    identity_second: str | None,
) -> str:
    if identity_first and identity_second:
        identity_note = f"The first response is from {identity_first}; the second is from {identity_second}."
    else:
        identity_note = "You are not told which model produced which response."

    return f"""You are comparing two responses to the same prompt and picking the better one.

PROMPT:
{prompt}

FIRST RESPONSE:
{response_first}

SECOND RESPONSE:
{response_second}

{identity_note}

Respond with ONLY a JSON object, no other text, in this exact shape:
{{"winner": "first" | "second" | "tie", "rationale": "<short reason>"}}"""


def parse_pairwise_judge_response(raw: str) -> PairwiseVerdict:
    data = _extract_json(raw)
    winner = data["winner"]
    if winner not in ("first", "second", "tie"):
        raise ValueError(f"judge response has invalid winner '{winner}': {raw}")
    return PairwiseVerdict(winner=winner, rationale=str(data["rationale"]))


_CODE_FENCE = re.compile(r"```(?:json)?\s*(.*?)\s*```", re.DOTALL)


def _extract_json(raw: str) -> dict:
    """Models frequently wrap JSON in a markdown code fence despite being asked not to - strip it before parsing."""
    fenced = _CODE_FENCE.search(raw)
    text = fenced.group(1) if fenced else raw
    try:
        return json.loads(text)
    except json.JSONDecodeError as e:
        raise ValueError(f"judge response was not valid JSON: {raw}") from e
