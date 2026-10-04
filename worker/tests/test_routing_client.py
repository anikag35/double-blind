import pytest

from worker.fake_client import FakeClient
from worker.routing_client import RoutingClient


def make_counting_factory():
    calls = {"count": 0}

    def factory():
        calls["count"] += 1
        return FakeClient()

    return factory, calls


def test_dispatches_to_the_client_matching_the_model_prefix():
    claude_factory, claude_calls = make_counting_factory()
    gpt_factory, gpt_calls = make_counting_factory()
    router = RoutingClient({"claude": claude_factory, "gpt": gpt_factory})

    router.generate("claude-opus-5", "hello")
    assert claude_calls["count"] == 1
    assert gpt_calls["count"] == 0

    router.generate("gpt-5", "hello")
    assert claude_calls["count"] == 1
    assert gpt_calls["count"] == 1


def test_provider_client_is_constructed_lazily():
    claude_factory, claude_calls = make_counting_factory()
    gpt_factory, gpt_calls = make_counting_factory()
    RoutingClient({"claude": claude_factory, "gpt": gpt_factory})

    # Constructing the router must not touch either factory.
    assert claude_calls["count"] == 0
    assert gpt_calls["count"] == 0


def test_provider_client_is_cached_across_calls():
    claude_factory, claude_calls = make_counting_factory()
    router = RoutingClient({"claude": claude_factory})

    router.generate("claude-opus-5", "one")
    router.generate("claude-sonnet-5", "two")

    assert claude_calls["count"] == 1, "the factory should only run once, not once per call"


def test_unmapped_model_prefix_raises():
    router = RoutingClient({"claude": FakeClient})
    with pytest.raises(ValueError):
        router.generate("gemini-pro", "hello")


def test_judge_rubric_and_judge_pairwise_also_dispatch_correctly():
    from worker.rubric import Criterion, Rubric

    router = RoutingClient({"claude": FakeClient})
    r = Rubric(scale="1-5", criteria=[Criterion(name="correctness", description="x", weight=1.0)])

    scores = router.judge_rubric("claude-opus-5", "prompt", "response", r)
    assert "correctness" in scores

    verdict = router.judge_pairwise("claude-opus-5", "prompt", "resp a", "resp b")
    assert verdict.winner in ("first", "second", "tie")
