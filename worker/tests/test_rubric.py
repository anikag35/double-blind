import pytest

from worker.rubric import parse_rubric


def test_parses_rubric_matching_design_doc_example():
    content = """
scale: 1-5
criteria:
  - name: correctness
    description: Is the response factually and technically accurate?
    weight: 1
  - name: completeness
    description: Does the response fully address the prompt?
    weight: 1
"""
    rubric = parse_rubric(content)
    assert rubric.scale == "1-5"
    assert len(rubric.criteria) == 2
    assert rubric.criteria[0].name == "correctness"
    assert rubric.criteria[0].weight == 1.0


def test_rejects_rubric_with_no_criteria():
    with pytest.raises(ValueError):
        parse_rubric("scale: 1-5\ncriteria: []\n")


def test_rejects_malformed_yaml():
    with pytest.raises(Exception):
        parse_rubric("not: valid: yaml: at: all: [")


def test_rejects_missing_scale():
    with pytest.raises(ValueError):
        parse_rubric("criteria:\n  - name: x\n    description: y\n    weight: 1\n")
