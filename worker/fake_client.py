import hashlib

from worker.client import CriterionResult, PairwiseVerdict
from worker.rubric import Rubric


def _stable_score(seed: str, low: int, high: int) -> int:
    """Deterministically derives an int in [low, high] from seed, the same way the scheduler's model_a_shown_first derives a bool from a task_id: hash the seed, use the hash's bytes as the source of pseudo-randomness."""
    digest = hashlib.sha256(seed.encode("utf-8")).digest()
    span = high - low + 1
    return low + (digest[0] % span)


class FakeClient:
    """Deterministic, no-network stand-in for a real model/judge client. Same input always produces the same output, so tests are reproducible."""

    def generate(self, prompt: str) -> str:
        return f"[fake response to: {prompt}]"

    def judge_rubric(self, prompt: str, response: str, rubric: Rubric) -> dict[str, CriterionResult]:
        results = {}
        for criterion in rubric.criteria:
            seed = f"{prompt}|{response}|{criterion.name}"
            score = _stable_score(seed, 1, 5)
            results[criterion.name] = CriterionResult(
                score=score,
                rationale=f"[fake rationale for {criterion.name}]",
            )
        return results

    def judge_pairwise(self, prompt: str, response_a: str, response_b: str) -> PairwiseVerdict:
        seed = f"{prompt}|{response_a}|{response_b}"
        pick = _stable_score(seed, 0, 2)  # 0 -> a, 1 -> b, 2 -> tie
        winner = ("a", "b", "tie")[pick]
        return PairwiseVerdict(winner=winner, rationale=f"[fake pairwise rationale: {winner}]")
