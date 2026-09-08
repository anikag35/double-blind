from worker.fake_client import FakeClient
from worker.rubric import Criterion, Rubric


def rubric():
    return Rubric(
        scale="1-5",
        criteria=[
            Criterion(name="correctness", description="...", weight=1.0),
            Criterion(name="clarity", description="...", weight=1.0),
        ],
    )


def test_generate_is_deterministic():
    client = FakeClient()
    assert client.generate("prompt a") == client.generate("prompt a")


def test_generate_differs_by_prompt():
    client = FakeClient()
    assert client.generate("prompt a") != client.generate("prompt b")


def test_judge_rubric_is_deterministic_and_covers_every_criterion():
    client = FakeClient()
    r = rubric()
    result = client.judge_rubric("prompt", "response", r)
    assert set(result.keys()) == {"correctness", "clarity"}
    assert result == client.judge_rubric("prompt", "response", r)


def test_judge_rubric_scores_are_within_scale():
    client = FakeClient()
    result = client.judge_rubric("prompt", "response", rubric())
    for criterion_result in result.values():
        assert 1 <= criterion_result.score <= 5


def test_judge_pairwise_is_deterministic():
    client = FakeClient()
    a = client.judge_pairwise("prompt", "response a", "response b")
    b = client.judge_pairwise("prompt", "response a", "response b")
    assert a == b


def test_judge_pairwise_winner_is_valid():
    client = FakeClient()
    verdict = client.judge_pairwise("prompt", "response a", "response b")
    assert verdict.winner in ("a", "b", "tie")
