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
    assert client.generate("claude", "prompt a") == client.generate("claude", "prompt a")


def test_generate_differs_by_prompt():
    client = FakeClient()
    assert client.generate("claude", "prompt a") != client.generate("claude", "prompt b")


def test_generate_differs_by_model():
    client = FakeClient()
    assert client.generate("claude", "prompt a") != client.generate("gemini", "prompt a")


def test_judge_rubric_is_deterministic_and_covers_every_criterion():
    client = FakeClient()
    r = rubric()
    result = client.judge_rubric("gpt-5", "prompt", "response", r)
    assert set(result.keys()) == {"correctness", "clarity"}
    assert result == client.judge_rubric("gpt-5", "prompt", "response", r)


def test_judge_rubric_scores_are_within_scale():
    client = FakeClient()
    result = client.judge_rubric("gpt-5", "prompt", "response", rubric())
    for criterion_result in result.values():
        assert 1 <= criterion_result.score <= 5


def test_judge_pairwise_is_deterministic():
    client = FakeClient()
    a = client.judge_pairwise("gpt-5", "prompt", "response first", "response second")
    b = client.judge_pairwise("gpt-5", "prompt", "response first", "response second")
    assert a == b


def test_judge_pairwise_winner_is_a_position_not_an_identity():
    client = FakeClient()
    verdict = client.judge_pairwise("gpt-5", "prompt", "response first", "response second")
    assert verdict.winner in ("first", "second", "tie")


def test_identity_participates_in_the_pairwise_verdict():
    # Across several inputs, at least one blind/unblind pair must produce a different winner 
    # Otherwise identity_first/identity_second aren't actually being used, and the fake would silently ignore them forever
    client = FakeClient()
    prompts = [f"prompt {i}" for i in range(20)]
    any_diverged = False
    for p in prompts:
        blind = client.judge_pairwise("gpt-5", p, "resp first", "resp second")
        unblind = client.judge_pairwise(
            "gpt-5", p, "resp first", "resp second",
            identity_first="claude", identity_second="gemini",
        )
        if blind.winner != unblind.winner:
            any_diverged = True
    assert any_diverged