import pytest

from worker.judge_prompts import (
    build_pairwise_judge_prompt,
    build_rubric_judge_prompt,
    parse_pairwise_judge_response,
    parse_rubric_judge_response,
)
from worker.rubric import Criterion, Rubric


def rubric():
    return Rubric(
        scale="1-5",
        criteria=[
            Criterion(name="correctness", description="accurate?", weight=1.0),
            Criterion(name="clarity", description="clear?", weight=1.0),
        ],
    )


def test_build_rubric_judge_prompt_includes_prompt_response_and_criteria():
    text = build_rubric_judge_prompt("what is 2+2?", "4", rubric())
    assert "what is 2+2?" in text
    assert "correctness" in text
    assert "clarity" in text


def test_parse_rubric_judge_response_plain_json():
    raw = '{"correctness": {"score": 4, "rationale": "mostly right"}, "clarity": {"score": 5, "rationale": "clear"}}'
    result = parse_rubric_judge_response(raw, rubric())
    assert result["correctness"].score == 4
    assert result["correctness"].rationale == "mostly right"
    assert result["clarity"].score == 5


def test_parse_rubric_judge_response_strips_code_fence():
    raw = '```json\n{"correctness": {"score": 3, "rationale": "ok"}, "clarity": {"score": 3, "rationale": "ok"}}\n```'
    result = parse_rubric_judge_response(raw, rubric())
    assert result["correctness"].score == 3


def test_parse_rubric_judge_response_missing_criterion_raises():
    raw = '{"correctness": {"score": 4, "rationale": "x"}}'
    with pytest.raises(ValueError):
        parse_rubric_judge_response(raw, rubric())


def test_parse_rubric_judge_response_invalid_json_raises():
    with pytest.raises(ValueError):
        parse_rubric_judge_response("not json at all", rubric())


def test_build_pairwise_judge_prompt_hides_identity_when_blind():
    text = build_pairwise_judge_prompt("p", "resp a", "resp b", None, None)
    assert "claude" not in text.lower()
    assert "not told which model" in text


def test_build_pairwise_judge_prompt_reveals_identity_when_unblind():
    text = build_pairwise_judge_prompt("p", "resp a", "resp b", "claude", "gemini")
    assert "claude" in text
    assert "gemini" in text


def test_parse_pairwise_judge_response():
    raw = '{"winner": "second", "rationale": "clearer"}'
    verdict = parse_pairwise_judge_response(raw)
    assert verdict.winner == "second"
    assert verdict.rationale == "clearer"


def test_parse_pairwise_judge_response_invalid_winner_raises():
    raw = '{"winner": "claude", "rationale": "x"}'
    with pytest.raises(ValueError):
        parse_pairwise_judge_response(raw)
