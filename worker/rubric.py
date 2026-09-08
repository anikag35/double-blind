from dataclasses import dataclass

import yaml


@dataclass
class Criterion:
    name: str
    description: str
    weight: float


@dataclass
class Rubric:
    scale: str
    criteria: list[Criterion]


def parse_rubric(content: str) -> Rubric:
    """Parses a rubric.yaml file's content"""
    # safe_load refuses to execute arbitrary Python objects that plain
    # yaml.load would allow
    parsed = yaml.safe_load(content)
    if not isinstance(parsed, dict):
        raise ValueError("rubric file must be a YAML mapping")

    scale = parsed.get("scale")
    criteria_raw = parsed.get("criteria")
    if not scale or not isinstance(criteria_raw, list) or not criteria_raw:
        raise ValueError("rubric file must have a non-empty 'scale' and 'criteria' list")

    criteria = [
        Criterion(name=c["name"], description=c["description"], weight=float(c["weight"]))
        for c in criteria_raw
    ]
    return Rubric(scale=scale, criteria=criteria)