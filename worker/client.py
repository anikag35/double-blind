from dataclasses import dataclass
from typing import Protocol

from worker.rubric import Rubric


@dataclass
class CriterionResult:
    score: int
    rationale: str


@dataclass
class PairwiseVerdict:
    winner: str  # "a" | "b" | "tie"
    rationale: str


class Client(Protocol):
    def generate(self, prompt: str) -> str:
        """Calls the contestant model to generate a response to prompt."""
        ...

    def judge_rubric(self, prompt: str, response: str, rubric: Rubric) -> dict[str, CriterionResult]:
        """Scores response against every criterion in rubric, keyed by criterion name."""
        ...

    def judge_pairwise(self, prompt: str, response_a: str, response_b: str) -> PairwiseVerdict:
        """Compares two responses to the same prompt and picks a winner."""
        ...
