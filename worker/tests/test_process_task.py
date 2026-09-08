import tempfile
from dataclasses import dataclass

from worker import blind_pb2
from worker.client import PairwiseVerdict
from worker.fake_client import FakeClient
from worker.process_task import process_task


def write_rubric_file(content: str) -> str:
    f = tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False)
    f.write(content)
    f.close()
    return f.name


RUBRIC_YAML = """
scale: 1-5
criteria:
  - name: correctness
    description: accurate?
    weight: 2
  - name: clarity
    description: clear?
    weight: 1
"""


@dataclass
class FixedPairwiseClient:
    """Test double that always returns a fixed verdict, recording what identity args it was called with."""
    winner: str
    seen_identity_first: str | None = None
    seen_identity_second: str | None = None
    called: bool = False

    def generate(self, model: str, prompt: str) -> str:
        return f"response from {model}"

    def judge_rubric(self, model, prompt, response, rubric):
        raise NotImplementedError

    def judge_pairwise(self, model, prompt, response_first, response_second, identity_first=None, identity_second=None):
        self.called = True
        self.seen_identity_first = identity_first
        self.seen_identity_second = identity_second
        return PairwiseVerdict(winner=self.winner, rationale="fixed")


def rubric_task(rubric_path: str) -> blind_pb2.Task:
    return blind_pb2.Task(
        available=True,
        task_id="task_1",
        run_id="run_1",
        mode=blind_pb2.MODE_RUBRIC,
        judge="gpt-5",
        rubric_path=rubric_path,
        prompt="what is 2+2?",
        prompt_hash="ph1",
        model="claude",
    )


def pairwise_task(model_a_shown_first: bool, condition) -> blind_pb2.Task:
    return blind_pb2.Task(
        available=True,
        task_id="task_2",
        run_id="run_1",
        mode=blind_pb2.MODE_PAIRWISE,
        judge="gpt-5",
        prompt="what is 2+2?",
        prompt_hash="ph1",
        model_a="claude",
        model_b="gemini",
        condition=condition,
        model_a_shown_first=model_a_shown_first,
    )


def test_rubric_task_composite_matches_weighted_average():
    rubric_path = write_rubric_file(RUBRIC_YAML)
    task = rubric_task(rubric_path)
    client = FakeClient()

    result = process_task(task, client)

    # Recompute independently from the same (deterministic) client, rather
    # than hardcoding magic numbers derived from the hash.
    from worker.rubric import parse_rubric
    rubric = parse_rubric(open(rubric_path).read())
    response = client.generate(task.model, task.prompt)
    scores = client.judge_rubric(task.judge, task.prompt, response, rubric)
    expected = (scores["correctness"].score * 2 + scores["clarity"].score * 1) / 3

    assert result.task_id == "task_1"
    assert result.WhichOneof("outcome") == "rubric"
    assert result.rubric.composite_score == expected
    assert {c.name for c in result.rubric.criteria} == {"correctness", "clarity"}


def test_pairwise_first_position_maps_to_model_a_when_a_shown_first():
    task = pairwise_task(model_a_shown_first=True, condition=blind_pb2.CONDITION_BLIND)
    client = FixedPairwiseClient(winner="first")

    result = process_task(task, client)

    assert result.WhichOneof("outcome") == "pairwise"
    assert result.pairwise.verdict == blind_pb2.VERDICT_MODEL_A


def test_pairwise_first_position_maps_to_model_b_when_b_shown_first():
    task = pairwise_task(model_a_shown_first=False, condition=blind_pb2.CONDITION_BLIND)
    client = FixedPairwiseClient(winner="first")

    result = process_task(task, client)

    assert result.pairwise.verdict == blind_pb2.VERDICT_MODEL_B


def test_pairwise_tie_maps_regardless_of_position():
    task = pairwise_task(model_a_shown_first=True, condition=blind_pb2.CONDITION_BLIND)
    client = FixedPairwiseClient(winner="tie")

    result = process_task(task, client)

    assert result.pairwise.verdict == blind_pb2.VERDICT_TIE


def test_pairwise_blind_condition_never_reveals_identity():
    task = pairwise_task(model_a_shown_first=True, condition=blind_pb2.CONDITION_BLIND)
    client = FixedPairwiseClient(winner="first")

    process_task(task, client)

    assert client.called
    assert client.seen_identity_first is None
    assert client.seen_identity_second is None


def test_pairwise_unblind_condition_reveals_identity_in_position_order():
    task = pairwise_task(model_a_shown_first=False, condition=blind_pb2.CONDITION_UNBLIND)
    client = FixedPairwiseClient(winner="first")

    process_task(task, client)

    # model_b was shown first, so identity_first must be model_b, not model_a.
    assert client.seen_identity_first == "gemini"
    assert client.seen_identity_second == "claude"
