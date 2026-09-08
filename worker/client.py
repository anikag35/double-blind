from dataclasses import dataclass
from typing import Protocol

from worker.rubric import Rubric


@dataclass
class CriterionResult:
    score: int
    rationale: str


@dataclass
class PairwiseVerdict:
    winner: str  # "first" | "second" | "tie" - a position, never a model identity
    rationale: str


class Client(Protocol):
    def generate(self, model: str, prompt: str) -> str:
        """Calls model to generate a response to prompt."""
        ...

    def judge_rubric(self, model: str, prompt: str, response: str, rubric: Rubric) -> dict[str, CriterionResult]:
        """Uses model as judge to score response against every criterion in rubric, keyed by criterion name."""
        ...

    def judge_pairwise(
        self,
        model: str,
        prompt: str,
        response_first: str,
        response_second: str,
        identity_first: str | None = None,
        identity_second: str | None = None,
    ) -> PairwiseVerdict:
        """Uses model as judge to compare two positioned responses and pick a winner by position. identity_first/identity_second are only passed when unblinding - the judge must never see them in the blind condition."""
        ...
